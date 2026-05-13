use anyhow::Result;

pub fn run_uninstall(force: bool, dry_run: bool) -> Result<()> {
    let paths_to_remove = vec![
        dirs::config_dir().map(|d| d.join("jcode")),
        dirs::data_dir().map(|d| d.join("jcode")),
        dirs::cache_dir().map(|d| d.join("jcode")),
    ];

    println!("jcode-plus uninstall");
    println!("Paths to remove:");

    for path in &paths_to_remove {
        if let Some(p) = path {
            if p.exists() {
                println!("  {}", p.display());
            }
        }
    }

    if dry_run {
        println!("Dry run mode — nothing deleted.");
        return Ok(());
    }

    if !force {
        println!("\nAre you sure? (y/N): ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() != "y" {
            println!("Uninstall cancelled.");
            return Ok(());
        }
    }

    for path in &paths_to_remove {
        if let Some(p) = path {
            if p.exists() {
                std::fs::remove_dir_all(p)?;
                println!("Removed: {}", p.display());
            }
        }
    }

    println!("Uninstall complete.");
    Ok(())
}
