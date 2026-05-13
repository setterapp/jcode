use anyhow::Result;

pub fn run_completion(shell: Option<String>) -> Result<()> {
    let cmd = crate::product::command_name();
    let shell_name = shell.as_deref().unwrap_or("bash");

    match shell_name {
        "bash" => {
            println!("eval \"$({cmd} completion bash)\"");
            println!("# Add the above to your ~/.bashrc");
        }
        "zsh" => {
            println!("eval \"$({cmd} completion zsh)\"");
            println!("# Add the above to your ~/.zshrc");
        }
        "fish" => {
            println!("{cmd} completion fish | source");
            println!("# Add the above to your ~/.config/fish/config.fish");
        }
        "powershell" => {
            println!("{cmd} completion powershell | Out-String | Invoke-Expression");
            println!("# Add the above to your PowerShell profile");
        }
        other => {
            println!("Shell completion for '{}' is not supported.", other);
            println!("Supported shells: bash, zsh, fish, powershell, elvish");
        }
    }

    Ok(())
}
