//! Git command execution engine supporting multiple cloning strategies.
//!
//! Handles standard full clones, shallow depth-1 clones, blobless clones (`--filter=blob:none`),
//! treeless clones (`--filter=tree:0`), and subfolder sparse checkouts.

use crate::types::CloneMethod;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::process::Command;

/// Helper to execute a `tokio::process::Command`, stream/capture stdout and stderr into `log_output`,
/// and ensure the command exits successfully.
async fn run_git_command(
    mut cmd: Command,
    step_description: &str,
    log_output: &mut String,
) -> Result<()> {
    let output = cmd
        .output()
        .await
        .with_context(|| format!("Failed to execute {}", step_description))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    log_output.push_str(&stdout);
    log_output.push_str(&stderr);

    if !output.status.success() {
        anyhow::bail!("{} failed:\n{}", step_description, log_output);
    }

    Ok(())
}

/// Clones a remote git repository using the specified clone strategy and options.
///
/// Returns captured logs/output from the git operations.
pub async fn execute_clone(
    repo_url: &str,
    target_dir: &str,
    branch: &str,
    method: CloneMethod,
    sparse_path: Option<&str>,
) -> Result<String> {
    let mut log_output = String::new();

    match method {
        CloneMethod::Normal => {
            let mut cmd = Command::new("git");
            cmd.arg("clone");
            if !branch.is_empty() {
                cmd.args(["--single-branch", "--branch", branch]);
            }
            cmd.arg(repo_url).arg(target_dir);

            run_git_command(cmd, "git clone", &mut log_output).await?;
        }
        CloneMethod::Shallow => {
            let mut cmd = Command::new("git");
            cmd.args(["clone", "--depth", "1"]);
            if !branch.is_empty() {
                cmd.args(["--single-branch", "-b", branch]);
            }
            cmd.arg(repo_url).arg(target_dir);

            run_git_command(cmd, "shallow git clone", &mut log_output).await?;
        }
        CloneMethod::Blobless => {
            let mut cmd = Command::new("git");
            cmd.args(["clone", "--filter=blob:none"]);
            if !branch.is_empty() {
                cmd.args(["--single-branch", "-b", branch]);
            }
            cmd.arg(repo_url).arg(target_dir);

            run_git_command(cmd, "blobless git clone", &mut log_output).await?;
        }
        CloneMethod::Treeless => {
            let mut cmd = Command::new("git");
            cmd.args(["clone", "--no-checkout", "--filter=tree:0"]);
            if !branch.is_empty() {
                cmd.args(["--single-branch", "-b", branch]);
            }
            cmd.arg(repo_url).arg(target_dir);

            run_git_command(cmd, "treeless git clone", &mut log_output).await?;
        }
        CloneMethod::Sparse => {
            let path_to_clone = sparse_path.unwrap_or("");
            if path_to_clone.is_empty() {
                anyhow::bail!("Sparse checkout requires a target subfolder path.");
            }

            // Step 1: Clone no-checkout blobless
            let mut clone_cmd = Command::new("git");
            clone_cmd.args(["clone", "--no-checkout", "--filter=blob:none"]);
            clone_cmd.arg(repo_url).arg(target_dir);

            run_git_command(clone_cmd, "initial clone for sparse checkout", &mut log_output).await?;

            let repo_path = Path::new(target_dir);

            // Step 2: Set sparse-checkout --no-cone
            let mut set_cmd = Command::new("git");
            set_cmd
                .current_dir(repo_path)
                .args(["sparse-checkout", "set", "--no-cone"]);

            run_git_command(set_cmd, "sparse-checkout set --no-cone", &mut log_output).await?;

            // Step 3: Add directory to sparse-checkout
            let mut add_cmd = Command::new("git");
            add_cmd
                .current_dir(repo_path)
                .args(["sparse-checkout", "add", "!/*", path_to_clone]);

            run_git_command(add_cmd, "sparse-checkout add directory", &mut log_output).await?;

            // Step 4: Checkout target branch (or HEAD/main)
            let target_branch = if !branch.is_empty() {
                branch
            } else {
                "HEAD"
            };

            let mut checkout_cmd = Command::new("git");
            checkout_cmd
                .current_dir(repo_path)
                .args(["checkout", target_branch]);

            run_git_command(checkout_cmd, "sparse-checkout branch checkout", &mut log_output).await?;
        }
    }

    Ok(log_output)
}
