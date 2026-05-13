use anyhow::Result;
use crate::stats;

pub fn run_stats(json: bool) -> Result<()> {
    let s = stats::load_stats()?;

    if json {
        println!("{}", serde_json::to_string_pretty(&s)?);
    } else {
        println!("Stats:");
        println!("  Total sessions:   {}", s.total_sessions);
        println!("  Total messages:   {}", s.total_messages);
        println!("  Input tokens:     {}", s.total_tokens_input);
        println!("  Output tokens:    {}", s.total_tokens_output);
        println!("  Cache tokens:     {}", s.total_tokens_cache);
        println!("  Total cost (USD): ${:.4}", s.total_cost_usd);

        if !s.by_model.is_empty() {
            println!("\nBy model:");
            for m in &s.by_model {
                println!("  {}: {} sessions, ${:.4}", m.model, m.sessions, m.cost_usd);
            }
        }
    }

    Ok(())
}
