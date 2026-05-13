use jcode_protocol::ServerEvent;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionListItem {
    pub id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TranscriptState {
    pub session_id: Option<String>,
    pub text: String,
    pub last_error: Option<String>,
    pub is_reloading: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolActivityItem {
    pub id: String,
    pub name: String,
    pub status: ToolStatus,
    pub output: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Started,
    Running,
    Done,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientCoreState {
    pub transcript: TranscriptState,
    pub tools: Vec<ToolActivityItem>,
    pub sessions: Vec<SessionListItem>,
}

impl ClientCoreState {
    pub fn apply_event(&mut self, event: &ServerEvent) {
        match event {
            ServerEvent::SessionId { session_id } => {
                self.transcript.session_id = Some(session_id.clone());
            }
            ServerEvent::TextDelta { text } => {
                self.transcript.text.push_str(text);
            }
            ServerEvent::TextReplace { text } => {
                self.transcript.text = text.clone();
            }
            ServerEvent::Error { message, .. } => {
                self.transcript.last_error = Some(message.clone());
            }
            ServerEvent::ToolStart { id, name } => self.upsert_tool(id, name, ToolStatus::Started),
            ServerEvent::ToolExec { id, name } => self.upsert_tool(id, name, ToolStatus::Running),
            ServerEvent::ToolDone {
                id,
                name,
                output,
                error,
            } => {
                self.upsert_tool(id, name, ToolStatus::Done);
                if let Some(tool) = self.tools.iter_mut().find(|t| t.id == *id) {
                    tool.output = Some(output.clone());
                    tool.error = error.clone();
                }
            }
            ServerEvent::History { all_sessions, .. } => {
                self.sessions = all_sessions
                    .iter()
                    .cloned()
                    .map(|id| SessionListItem { id })
                    .collect();
            }
            ServerEvent::Reloading { .. } => {
                self.transcript.is_reloading = true;
            }
            _ => {}
        }
    }

    fn upsert_tool(&mut self, id: &str, name: &str, status: ToolStatus) {
        if let Some(tool) = self.tools.iter_mut().find(|t| t.id == id) {
            tool.status = status;
            tool.name = name.to_string();
            return;
        }
        self.tools.push(ToolActivityItem {
            id: id.to_string(),
            name: name.to_string(),
            status,
            output: None,
            error: None,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reducer_tracks_streaming_text_and_replace() {
        let mut state = ClientCoreState::default();
        state.apply_event(&ServerEvent::TextDelta {
            text: "hello ".to_string(),
        });
        state.apply_event(&ServerEvent::TextDelta {
            text: "world".to_string(),
        });
        assert_eq!(state.transcript.text, "hello world");

        state.apply_event(&ServerEvent::TextReplace {
            text: "clean".to_string(),
        });
        assert_eq!(state.transcript.text, "clean");
    }

    #[test]
    fn reducer_tracks_tool_lifecycle() {
        let mut state = ClientCoreState::default();
        state.apply_event(&ServerEvent::ToolStart {
            id: "t1".to_string(),
            name: "web.search".to_string(),
        });
        state.apply_event(&ServerEvent::ToolExec {
            id: "t1".to_string(),
            name: "web.search".to_string(),
        });
        state.apply_event(&ServerEvent::ToolDone {
            id: "t1".to_string(),
            name: "web.search".to_string(),
            output: "{\"ok\":true}".to_string(),
            error: None,
        });

        assert_eq!(state.tools.len(), 1);
        assert_eq!(state.tools[0].status, ToolStatus::Done);
        assert_eq!(state.tools[0].output.as_deref(), Some("{\"ok\":true}"));
    }

    #[test]
    fn reducer_tracks_session_list_from_history_snapshot() {
        let mut state = ClientCoreState::default();
        state.apply_event(&ServerEvent::History {
            id: 1,
            session_id: "session_a".to_string(),
            messages: Vec::new(),
            images: Vec::new(),
            provider_name: None,
            provider_model: None,
            available_models: Vec::new(),
            available_model_routes: Vec::new(),
            mcp_servers: Vec::new(),
            skills: Vec::new(),
            total_tokens: None,
            all_sessions: vec!["session_a".to_string(), "session_b".to_string()],
            client_count: None,
            is_canary: None,
            server_version: None,
            server_name: None,
            server_icon: None,
            server_has_update: None,
            was_interrupted: None,
            reload_recovery: None,
            connection_type: None,
            status_detail: None,
            upstream_provider: None,
            reasoning_effort: None,
            service_tier: None,
            subagent_model: None,
            autoreview_enabled: None,
            autojudge_enabled: None,
            compaction_mode: jcode_config_types::CompactionMode::default(),
            activity: None,
            side_panel: jcode_side_panel_types::SidePanelSnapshot::default(),
        });

        assert_eq!(state.sessions.len(), 2);
        assert_eq!(state.sessions[0].id, "session_a");
        assert_eq!(state.sessions[1].id, "session_b");
    }

    #[test]
    fn reducer_marks_reloading_state() {
        let mut state = ClientCoreState::default();
        assert!(!state.transcript.is_reloading);
        state.apply_event(&ServerEvent::Reloading { new_socket: None });
        assert!(state.transcript.is_reloading);
    }
}

