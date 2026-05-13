use anyhow::Result;
use std::path::PathBuf;

pub struct AgentBuilder;

impl AgentBuilder {
    pub fn list_agents() -> Result<Vec<AgentSummary>> {
        let agent_dirs = Self::agent_search_paths();
        let mut agents = Vec::new();

        for dir in agent_dirs {
            if !dir.exists() {
                continue;
            }
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Ok(agent) = Self::parse_agent_file(&path) {
                        agents.push(agent);
                    }
                }
            }
        }

        Ok(agents)
    }

    pub async fn create_agent(
        name: &str,
        description: &str,
        model: Option<&str>,
        permissions: Option<&str>,
    ) -> Result<PathBuf> {
        let agent_dir = Self::default_agents_dir()?;
        std::fs::create_dir_all(&agent_dir)?;

        let file_path = agent_dir.join(format!("{}.md", name.to_lowercase().replace(' ', "-")));

        let mut content = String::new();
        content.push_str("---\n");
        content.push_str(&format!("description: \"{}\"\n", description));
        content.push_str("mode: subagent\n");
        if let Some(m) = model {
            content.push_str(&format!("model: {}\n", m));
        }
        if let Some(p) = permissions {
            content.push_str(&format!("permission:\n  {}\n", p));
        }
        content.push_str("---\n\n");
        content.push_str(&format!("# {}\n\n", name));
        content.push_str("You are a specialized agent. ");
        content.push_str(&description);
        content.push('\n');

        std::fs::write(&file_path, content)?;
        Ok(file_path)
    }

    fn agent_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if let Ok(cwd) = std::env::current_dir() {
            paths.push(cwd.join("agents"));
            paths.push(cwd.join(".opencode").join("agents"));
        }
        if let Some(config) = dirs::config_dir() {
            paths.push(config.join("jcode").join("agents"));
        }
        paths
    }

    fn default_agents_dir() -> Result<PathBuf> {
        let config = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;
        Ok(config.join("jcode").join("agents"))
    }

    fn parse_agent_file(path: &PathBuf) -> Result<AgentSummary> {
        let content = std::fs::read_to_string(path)?;
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut description = String::new();
        let mut model = None;

        if let Some(fm_end) = content.find("---\n") {
            if content.starts_with("---\n") {
                let frontmatter = &content[4..fm_end];
                for line in frontmatter.lines() {
                    if let Some(desc) = line.strip_prefix("description: \"") {
                        description = desc.trim_end_matches('"').to_string();
                    } else if let Some(m) = line.strip_prefix("model: ") {
                        model = Some(m.to_string());
                    }
                }
            }
        }

        Ok(AgentSummary {
            name,
            description,
            model,
            path: path.clone(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AgentSummary {
    pub name: String,
    pub description: String,
    pub model: Option<String>,
    pub path: PathBuf,
}
