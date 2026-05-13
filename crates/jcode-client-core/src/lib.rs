use jcode_protocol::ServerEvent;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionListItem {
    pub id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionMetaState {
    pub provider_name: Option<String>,
    pub provider_model: Option<String>,
    pub available_models: Vec<String>,
    pub connection_type: Option<String>,
    pub connection_phase: Option<String>,
    pub status_detail: Option<String>,
    pub upstream_provider: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingStdinRequest {
    pub request_id: String,
    pub prompt: String,
    pub is_password: bool,
    pub tool_call_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClientCoreState {
    pub transcript: TranscriptState,
    pub tools: Vec<ToolActivityItem>,
    pub sessions: Vec<SessionListItem>,
    pub session_meta: SessionMetaState,
    pub pending_stdin_request: Option<PendingStdinRequest>,
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
            ServerEvent::History {
                all_sessions,
                provider_name,
                provider_model,
                available_models,
                ..
            } => {
                self.sessions = all_sessions
                    .iter()
                    .cloned()
                    .map(|id| SessionListItem { id })
                    .collect();
                self.session_meta.provider_name = provider_name.clone();
                self.session_meta.provider_model = provider_model.clone();
                self.session_meta.available_models = available_models.clone();
            }
            ServerEvent::ConnectionType { connection } => {
                self.session_meta.connection_type = Some(connection.clone());
            }
            ServerEvent::ConnectionPhase { phase } => {
                self.session_meta.connection_phase = Some(phase.clone());
            }
            ServerEvent::StatusDetail { detail } => {
                self.session_meta.status_detail = Some(detail.clone());
            }
            ServerEvent::UpstreamProvider { provider } => {
                self.session_meta.upstream_provider = Some(provider.clone());
            }
            ServerEvent::StdinRequest {
                request_id,
                prompt,
                is_password,
                tool_call_id,
            } => {
                self.pending_stdin_request = Some(PendingStdinRequest {
                    request_id: request_id.clone(),
                    prompt: prompt.clone(),
                    is_password: *is_password,
                    tool_call_id: tool_call_id.clone(),
                });
            }
            ServerEvent::Done { .. } => {
                self.pending_stdin_request = None;
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

    #[test]
    fn reducer_tracks_session_meta_and_connection_state() {
        let mut state = ClientCoreState::default();
        state.apply_event(&ServerEvent::History {
            id: 1,
            session_id: "session_a".to_string(),
            messages: Vec::new(),
            images: Vec::new(),
            provider_name: Some("openai".to_string()),
            provider_model: Some("gpt-5.5".to_string()),
            available_models: vec!["gpt-5.5".to_string(), "gpt-5.4".to_string()],
            available_model_routes: Vec::new(),
            mcp_servers: Vec::new(),
            skills: Vec::new(),
            total_tokens: None,
            all_sessions: vec!["session_a".to_string()],
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
        state.apply_event(&ServerEvent::ConnectionType {
            connection: "websocket".to_string(),
        });
        state.apply_event(&ServerEvent::ConnectionPhase {
            phase: "authenticating".to_string(),
        });
        state.apply_event(&ServerEvent::StatusDetail {
            detail: "warming transport".to_string(),
        });
        state.apply_event(&ServerEvent::UpstreamProvider {
            provider: "openai".to_string(),
        });

        assert_eq!(state.session_meta.provider_name.as_deref(), Some("openai"));
        assert_eq!(state.session_meta.provider_model.as_deref(), Some("gpt-5.5"));
        assert_eq!(state.session_meta.available_models.len(), 2);
        assert_eq!(
            state.session_meta.connection_type.as_deref(),
            Some("websocket")
        );
        assert_eq!(
            state.session_meta.connection_phase.as_deref(),
            Some("authenticating")
        );
        assert_eq!(
            state.session_meta.status_detail.as_deref(),
            Some("warming transport")
        );
        assert_eq!(
            state.session_meta.upstream_provider.as_deref(),
            Some("openai")
        );
    }

    #[test]
    fn reducer_tracks_and_clears_pending_stdin_request() {
        let mut state = ClientCoreState::default();
        state.apply_event(&ServerEvent::StdinRequest {
            request_id: "req_123".to_string(),
            prompt: "Password: ".to_string(),
            is_password: true,
            tool_call_id: "tool_1".to_string(),
        });

        let pending = state
            .pending_stdin_request
            .as_ref()
            .expect("stdin request should be tracked");
        assert_eq!(pending.request_id, "req_123");
        assert_eq!(pending.prompt, "Password: ");
        assert!(pending.is_password);
        assert_eq!(pending.tool_call_id, "tool_1");

        state.apply_event(&ServerEvent::Done { id: 9 });
        assert!(state.pending_stdin_request.is_none());
    }
}
