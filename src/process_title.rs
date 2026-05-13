use crate::cli::args::{AmbientCommand, Args, Command};

const LINUX_PROCESS_TITLE_LIMIT: usize = 15;
fn product_name() -> &'static str {
    crate::product::binary_stem()
}

fn compact_process_title(prefix: &str, name: Option<&str>) -> String {
    let mut title = prefix.to_string();
    if let Some(name) = name.filter(|name| !name.is_empty()) {
        let remaining = LINUX_PROCESS_TITLE_LIMIT.saturating_sub(title.len());
        if remaining > 0 {
            title.push_str(&name.chars().take(remaining).collect::<String>());
        }
    }
    title
}

pub(crate) fn session_name(session_id: &str) -> String {
    crate::id::extract_session_name(session_id)
        .map(|name| name.to_string())
        .unwrap_or_else(|| session_id.to_string())
}

fn normalized_display_title(title: &str) -> Option<String> {
    let normalized = title.split_whitespace().collect::<Vec<_>>().join(" ");
    (!normalized.is_empty()).then_some(normalized)
}

fn capitalize_ascii_label(label: &str) -> String {
    let mut chars = label.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let truncated: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{}…", truncated)
    } else {
        truncated
    }
}

pub(crate) fn terminal_session_label(session_name: &str, display_title: Option<&str>) -> String {
    let fallback = capitalize_ascii_label(session_name);
    let Some(title) = display_title.and_then(normalized_display_title) else {
        return fallback;
    };
    if title.eq_ignore_ascii_case(session_name) || title.eq_ignore_ascii_case(&fallback) {
        return fallback;
    }
    format!("{} ({})", truncate_chars(&title, 48), session_name)
}

pub(crate) fn terminal_session_label_for_id(session_id: &str) -> String {
    let session_name = session_name(session_id);
    let display_title = crate::session::Session::load_startup_stub(session_id)
        .ok()
        .and_then(|session| session.display_title().map(ToOwned::to_owned));
    match display_title.as_deref() {
        Some(title) => terminal_session_label(&session_name, Some(title)),
        None => session_name,
    }
}

pub(crate) fn set_title(title: impl AsRef<str>) {
    proctitle::set_title(title.as_ref());
    set_killall_process_name();
}

fn set_killall_process_name() {
    #[cfg(target_os = "linux")]
    unsafe {
        let mut name = [0u8; 16];
        let bytes = product_name().as_bytes();
        let len = bytes.len().min(name.len().saturating_sub(1));
        name[..len].copy_from_slice(&bytes[..len]);
        let _ = libc::prctl(libc::PR_SET_NAME, name.as_ptr(), 0, 0, 0);
    }
}

pub(crate) fn set_server_title(server_name: &str) {
    set_title(compact_process_title(
        &format!("{}:s:", product_name()),
        Some(server_name),
    ));
}

pub(crate) fn set_client_generic_title(is_selfdev: bool) {
    let prefix = if is_selfdev {
        format!("{}:selfdev", product_name())
    } else {
        format!("{}:client", product_name())
    };
    set_title(compact_process_title(&prefix, None));
}

pub(crate) fn set_client_session_title(session_id: &str, is_selfdev: bool) {
    set_client_display_title(&session_name(session_id), is_selfdev);
}

pub(crate) fn set_client_display_title(session_name: &str, is_selfdev: bool) {
    let prefix = if is_selfdev {
        format!("{}:d:", product_name())
    } else {
        format!("{}:c:", product_name())
    };
    set_title(compact_process_title(&prefix, Some(session_name)));
}

pub(crate) fn set_client_remote_display_title(
    server_name: &str,
    session_name: &str,
    is_selfdev: bool,
) {
    if server_name.is_empty() || server_name.eq_ignore_ascii_case(product_name()) {
        set_client_display_title(session_name, is_selfdev);
        return;
    }
    let prefix = if is_selfdev {
        format!("{}:d:", product_name())
    } else {
        format!("{}:c:", product_name())
    };
    set_title(format!("{prefix}{server_name}/{session_name}"));
}

pub(crate) fn initial_title(args: &Args) -> String {
    let product = product_name();
    match &args.command {
        Some(Command::Serve { .. }) => format!("{product}:server"),
        Some(Command::Connect) => format!("{product}:client"),
        Some(Command::Run { .. }) => format!("{product} run"),
        Some(Command::Login { .. }) => format!("{product} login"),
        Some(Command::Repl) => format!("{product} repl"),
        Some(Command::Update) => format!("{product} update"),
        Some(Command::Version { .. }) => format!("{product} version"),
        Some(Command::Usage { .. }) => format!("{product} usage"),
        Some(Command::SelfDev { .. }) => format!("{product}:selfdev"),
        Some(Command::Debug { .. }) => format!("{product} debug"),
        Some(Command::Auth(_)) => format!("{product} auth"),
        Some(Command::Provider(_)) => format!("{product} provider"),
        Some(Command::Memory(_)) => format!("{product} memory"),
        Some(Command::Session(_)) => format!("{product} session"),
        Some(Command::Ambient(subcommand)) => match subcommand {
            AmbientCommand::RunVisible => format!("{product} ambient visible"),
            _ => format!("{product} ambient"),
        },
        Some(Command::Pair { .. }) => format!("{product} pair"),
        Some(Command::Permissions) => format!("{product} permissions"),
        Some(Command::Transcript { .. }) => format!("{product} transcript"),
        Some(Command::Dictate { .. }) => format!("{product} dictate"),
        Some(Command::SetupHotkey {
            listen_macos_hotkey,
        }) => {
            if *listen_macos_hotkey {
                format!("{product} hotkey listener")
            } else {
                format!("{product} hotkey setup")
            }
        }
        Some(Command::Browser { .. }) => format!("{product} browser"),
        Some(Command::Replay { .. }) => format!("{product} replay"),
        Some(Command::Model(_)) => format!("{product} model"),
        Some(Command::AuthTest { .. }) => format!("{product} auth-test"),
        Some(Command::Restart { .. }) => format!("{product} restart"),
        Some(Command::SetupLauncher) => format!("{product} setup-launcher"),
        Some(Command::Mcp(_)) => format!("{product} mcp"),
        Some(Command::AgentCmd(_)) => format!("{product} agent"),
        Some(Command::Plug { .. }) => format!("{product} plug"),
        Some(Command::ServeWeb { .. }) => format!("{product} serve"),
        Some(Command::Web { .. }) => format!("{product} web"),
        Some(Command::Export { .. }) => format!("{product} export"),
        Some(Command::Import { .. }) => format!("{product} import"),
        Some(Command::ImportFromJcode { .. }) => format!("{product} import-from-jcode"),
        Some(Command::Github(_)) => format!("{product} github"),
        Some(Command::Pr { .. }) => format!("{product} pr"),
        Some(Command::Stats { .. }) => format!("{product} stats"),
        Some(Command::Uninstall { .. }) => format!("{product} uninstall"),
        Some(Command::Db { .. }) => format!("{product} db"),
        Some(Command::Completion { .. }) => format!("{product} completion"),
        None => {
            if let Some(resume) = args.resume.as_deref().filter(|resume| !resume.is_empty()) {
                let prefix = if crate::cli::selfdev::client_selfdev_requested() {
                    format!("{product}:d:")
                } else {
                    format!("{product}:c:")
                };
                compact_process_title(&prefix, Some(&session_name(resume)))
            } else if crate::cli::selfdev::client_selfdev_requested() {
                format!("{product}:selfdev")
            } else {
                format!("{product}:client")
            }
        }
    }
}

pub(crate) fn set_initial_title(args: &Args) {
    set_title(initial_title(args));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::Args;
    use crate::storage::lock_test_env;
    use clap::Parser;

    const SELFDEV_ENV: &str = crate::cli::selfdev::CLIENT_SELFDEV_ENV;

    fn with_selfdev_env_removed<T>(f: impl FnOnce() -> T) -> T {
        let _guard = lock_test_env();
        let previous = std::env::var_os(SELFDEV_ENV);
        crate::env::remove_var(SELFDEV_ENV);
        let result = f();
        if let Some(value) = previous {
            crate::env::set_var(SELFDEV_ENV, value);
        }
        result
    }

    #[test]
    fn initial_title_labels_server() {
        with_selfdev_env_removed(|| {
            let args = Args::parse_from(["jcode", "serve"]);
            assert_eq!(initial_title(&args), "jcode:server");
        });
    }

    #[test]
    fn initial_title_labels_resume_client_with_short_name() {
        with_selfdev_env_removed(|| {
            let args = Args::parse_from(["jcode", "--resume", "session_fox_123"]);
            assert_eq!(initial_title(&args), "jcode:c:fox");
        });
    }

    #[test]
    fn terminal_session_label_includes_custom_title_and_short_name() {
        assert_eq!(
            terminal_session_label("fox", Some("Release planning")),
            "Release planning (fox)"
        );
        assert_eq!(terminal_session_label("fox", Some("Fox")), "Fox");
        assert_eq!(terminal_session_label("fox", None), "Fox");
    }

    #[test]
    fn terminal_session_label_for_id_reads_custom_title_from_session() {
        let _guard = lock_test_env();
        let previous_home = std::env::var_os("JCODE_HOME");
        let temp = tempfile::tempdir().expect("temp dir");
        crate::env::set_var("JCODE_HOME", temp.path());

        let mut session = crate::session::Session::create_with_id(
            "session_fox_123".to_string(),
            None,
            Some("Generated title".to_string()),
        );
        session.rename_title(Some("Release planning".to_string()));
        session.save().expect("save session");

        assert_eq!(
            terminal_session_label_for_id("session_fox_123"),
            "Release planning (fox)"
        );

        if let Some(previous_home) = previous_home {
            crate::env::set_var("JCODE_HOME", previous_home);
        } else {
            crate::env::remove_var("JCODE_HOME");
        }
    }

    #[test]
    fn initial_title_labels_selfdev_command() {
        with_selfdev_env_removed(|| {
            let args = Args::parse_from(["jcode", "self-dev"]);
            assert_eq!(initial_title(&args), "jcode:selfdev");
        });
    }
}
