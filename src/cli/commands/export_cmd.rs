use anyhow::Result;
use crate::session::{self, Session};
use serde_json::json;

pub async fn run_export(session_ref: Option<String>, sanitize: bool, output: Option<String>) -> Result<()> {
    let session_id = match session_ref {
        Some(ref id) => session::find_session_by_name_or_id(id)?,
        None => anyhow::bail!("Session ID required. Use --resume to list sessions."),
    };

    let session = Session::load(&session_id)?;

    let display_name = session.display_name().to_string();
    let display_title = session.display_title().map(|s| s.to_string());

    let messages_val = if sanitize {
        // Keep message structure but redact content text
        session
            .messages
            .iter()
            .map(|msg| {
                json!({
                    "id": msg.id,
                    "role": msg.role,
                    "content": "[REDACTED]",
                    "display_role": msg.display_role,
                    "timestamp": msg.timestamp,
                })
            })
            .collect::<Vec<_>>()
    } else {
        session
            .messages
            .iter()
            .map(|msg| {
                json!({
                    "id": msg.id,
                    "role": msg.role,
                    "content": msg.content,
                    "display_role": msg.display_role,
                    "timestamp": msg.timestamp,
                    "tool_duration_ms": msg.tool_duration_ms,
                    "token_usage": msg.token_usage,
                })
            })
            .collect::<Vec<_>>()
    };

    let export_data = json!({
        "session_id": session.id,
        "title": session.title,
        "custom_title": session.custom_title,
        "display_name": display_name,
        "display_title": display_title,
        "created_at": session.created_at,
        "updated_at": session.updated_at,
        "message_count": session.messages.len(),
        "messages": messages_val,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "sanitized": sanitize,
        "provider_key": session.provider_key,
        "model": session.model,
        "reasoning_effort": session.reasoning_effort,
        "saved": session.saved,
        "save_label": session.save_label,
        "short_name": session.short_name,
        "status": session.status,
    });

    let json_str = serde_json::to_string_pretty(&export_data)?;

    match output {
        Some(path) => {
            std::fs::write(&path, &json_str)?;
            println!(
                "Exported session {} ({}) to {} ({} messages, sanitized: {})",
                display_name,
                session.id,
                path,
                session.messages.len(),
                sanitize
            );
        }
        None => {
            println!("{}", json_str);
        }
    }

    Ok(())
}

pub async fn run_import(input: &str) -> Result<()> {
    let content = if input.starts_with("http://") || input.starts_with("https://") {
        let resp = reqwest::get(input).await?;
        resp.text().await?
    } else {
        std::fs::read_to_string(input)?
    };

    let data: serde_json::Value = serde_json::from_str(&content)?;

    // Extract session_id from export format or generate one
    let session_id = data
        .get("session_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| crate::id::new_id("imported"));

    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Parse messages from the "messages" array
    let messages_raw = data
        .get("messages")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow::anyhow!("No 'messages' array found in import data"))?;

    let messages: Vec<crate::session::StoredMessage> = messages_raw
        .iter()
        .map(|m| serde_json::from_value(m.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| anyhow::anyhow!("Failed to parse message in import data: {}", e))?;

    let mut session = Session::create_with_id(session_id.clone(), None, title);
    session.messages = messages;
    session.status = session::SessionStatus::Closed;
    session.save()?;

    let display_name = session.display_name().to_string();
    let display_title = session.display_title().map(|s| s.to_string());

    match display_title {
        Some(ref title) => println!(
            "Imported session {} (\"{}\") with {} messages from {}",
            display_name,
            title,
            session.messages.len(),
            input
        ),
        None => println!(
            "Imported session {} with {} messages from {}",
            display_name,
            session.messages.len(),
            input
        ),
    }

    Ok(())
}
