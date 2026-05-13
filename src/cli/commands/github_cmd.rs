use anyhow::Result;
use crate::github::GithubIntegration;

pub async fn run_github_install() -> Result<()> {
    GithubIntegration::install()
}

pub async fn run_github_run() -> Result<()> {
    println!("GitHub Action mode not yet implemented.");
    Ok(())
}

pub async fn run_pr(number: &str) -> Result<()> {
    GithubIntegration::checkout_pr(number)?;
    println!("Ready to work on PR #{}", number);
    Ok(())
}
