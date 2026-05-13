use anyhow::Result;
use crate::agent_builder::AgentBuilder;

pub async fn run_agent_create(name: Option<String>) -> Result<()> {
    let agent_name = match name {
        Some(n) => n,
        None => {
            println!("Agent name: ");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            input.trim().to_string()
        }
    };

    if agent_name.is_empty() {
        anyhow::bail!("Agent name is required");
    }

    let path = AgentBuilder::create_agent(
        &agent_name,
        &format!("Custom agent: {}", agent_name),
        None,
        None,
    )
    .await?;

    println!("Created agent: {} -> {}", agent_name, path.display());
    Ok(())
}

pub async fn run_agent_list() -> Result<()> {
    let agents = AgentBuilder::list_agents()?;

    if agents.is_empty() {
        println!("No custom agents found.");
        println!("  Use: jcode agent create <name>");
        return Ok(());
    }

    println!("{:<20} {:<40} MODEL", "NAME", "DESCRIPTION");
    println!("{}", "-".repeat(80));
    for a in &agents {
        let model = a.model.as_deref().unwrap_or("default");
        println!("{:<20} {:<40} {}", a.name, a.description, model);
    }

    Ok(())
}
