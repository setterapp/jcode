use anyhow::Result;
use std::collections::HashMap;
use sha2::{Sha256, Digest};
use base64::Engine;

use crate::types::McpOAuthState;

pub struct McpOAuth {
    states: HashMap<String, McpOAuthState>,
}

impl McpOAuth {
    pub fn new() -> Self {
        Self { states: HashMap::new() }
    }

    pub fn generate_pkce_pair(&self) -> (String, String) {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use rand::Rng;
        let mut rng = rand::rng();
        let code_verifier: String = (0..64)
            .map(|_| {
                let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
                chars[rng.random_range(0..chars.len())] as char
            })
            .collect();

        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let code_challenge = URL_SAFE_NO_PAD.encode(hasher.finalize());

        (code_verifier, code_challenge)
    }

    pub fn start_flow(&mut self, server_name: &str, authorization_url: &str) -> Result<McpOAuthState> {
        let state = uuid::Uuid::new_v4().to_string();
        let (code_verifier, code_challenge) = self.generate_pkce_pair();

        let auth_state = McpOAuthState {
            server_name: server_name.to_string(),
            authorization_url: authorization_url.to_string(),
            state: state.clone(),
            code_verifier,
        };

        self.states.insert(state.clone(), auth_state.clone());
        Ok(auth_state)
    }

    pub fn complete_flow(&mut self, state: &str, _code: &str) -> Result<()> {
        self.states.remove(state);
        Ok(())
    }
}
