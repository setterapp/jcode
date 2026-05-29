use super::{Tool, ToolContext, ToolOutput};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct ToolSearchTool;

impl ToolSearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Deserialize)]
struct ToolSearchInput {
    query: String,
    #[serde(default = "default_max_results")]
    max_results: usize,
}

fn default_max_results() -> usize {
    5
}

// All tools available in jcode-plus with their descriptions.
const AVAILABLE_TOOLS: &[(&str, &str)] = &[
    ("Bash", "Execute shell commands"),
    ("Read", "Read files from the local filesystem"),
    ("Write", "Write files to the local filesystem"),
    ("Edit", "Perform exact string replacements in files"),
    ("Glob", "Fast file pattern matching"),
    ("Grep", "Search file contents with regex"),
    ("Agent", "Launch a sub-agent for complex multi-step tasks"),
    ("Skill", "Execute a skill within the main conversation"),
    ("WebFetch", "Fetch content from a URL"),
    ("WebSearch", "Search the web"),
    ("TodoWrite", "Create and manage a task list"),
    ("NotebookRead", "Read Jupyter notebooks"),
    ("NotebookEdit", "Edit Jupyter notebook cells"),
    ("ToolSearch", "Search available tool schemas"),
    ("ScheduleWakeup", "Schedule when to resume work in /loop dynamic mode"),
];

#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "tool_search"
    }

    fn description(&self) -> &str {
        "Fetches full schema definitions for deferred tools so they can be called."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Query to find tools. Use 'select:<name>' for direct selection, or keywords to search."
                },
                "max_results": {
                    "type": "number",
                    "default": 5,
                    "description": "Maximum number of results to return."
                }
            },
            "required": ["query"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, input: Value, _ctx: ToolContext) -> Result<ToolOutput> {
        let params: ToolSearchInput = serde_json::from_value(input)?;
        let query_lower = params.query.to_lowercase();

        // Handle "select:Tool1,Tool2" syntax
        let selected_names: Option<Vec<&str>> = if query_lower.starts_with("select:") {
            Some(
                params.query["select:".len()..]
                    .split(',')
                    .map(|s| s.trim())
                    .collect(),
            )
        } else {
            None
        };

        let matches: Vec<(&str, &str)> = if let Some(ref names) = selected_names {
            AVAILABLE_TOOLS
                .iter()
                .filter(|(name, _)| names.iter().any(|n| n.eq_ignore_ascii_case(name)))
                .map(|(n, d)| (*n, *d))
                .collect()
        } else {
            AVAILABLE_TOOLS
                .iter()
                .filter(|(name, desc)| {
                    name.to_lowercase().contains(&query_lower)
                        || desc.to_lowercase().contains(&query_lower)
                })
                .take(params.max_results)
                .map(|(n, d)| (*n, *d))
                .collect()
        };

        if matches.is_empty() {
            return Ok(ToolOutput::new(format!(
                "No tools matching '{}'. All tools are immediately available in jcode-plus — no deferred loading needed. Available: {}",
                params.query,
                AVAILABLE_TOOLS.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
            )));
        }

        let mut output = format!(
            "In jcode-plus all tools are immediately available — no deferred loading.\n\n"
        );
        output.push_str(&format!("Tools matching '{}':\n\n", params.query));

        for (name, desc) in &matches {
            output.push_str(&format!("<function>{{\n  \"name\": \"{}\",\n  \"description\": \"{}\"\n}}</function>\n", name, desc));
        }

        Ok(ToolOutput::new(output).with_title(format!("ToolSearch: {}", params.query)))
    }
}
