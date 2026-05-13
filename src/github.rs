use anyhow::Result;
use std::process::Command;

pub struct GithubIntegration;

impl GithubIntegration {
    pub fn install() -> Result<()> {
        let output = Command::new("gh")
            .args(["extension", "install", "opencode"])
            .output()
            .map_err(|e| anyhow::anyhow!("GitHub CLI (gh) not available: {}", e))?;

        if output.status.success() {
            println!("GitHub Actions integration installed successfully.");
            println!("Configure via: gh secret set OPENCODE_API_KEY");
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("Note: {}", stderr.trim());
            println!("Manual setup: https://opencode.ai/docs/github-actions");
        }

        Ok(())
    }

    pub fn check_gh_installed() -> bool {
        Command::new("gh").arg("--version").output().is_ok()
    }

    pub fn checkout_pr(number: &str) -> Result<String> {
        if !Self::check_gh_installed() {
            anyhow::bail!("GitHub CLI (gh) is not installed. Install it from https://cli.github.com");
        }

        let output = Command::new("gh")
            .args(["pr", "checkout", number])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Failed to checkout PR #{}: {}", number, stderr.trim());
        }

        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        println!("Checked out PR #{} -> {}", number, branch);
        Ok(branch)
    }
}
