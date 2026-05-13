use anyhow::Result;

pub fn run_db(query: Option<String>, format: &str) -> Result<()> {
    if let Some(q) = query {
        println!("Query: {}", q);
        println!("Format: {}", format);
        println!("(SQLite query execution not yet implemented in jcode-plus)");
    } else {
        println!("SQLite REPL");
        println!("(Interactive SQLite mode not yet implemented in jcode-plus)");
        println!("Use: jcode db \"SELECT * FROM sessions LIMIT 10\"");
    }

    Ok(())
}
