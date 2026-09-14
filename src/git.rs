use crate::types::CloneMethod;
use anyhow::{Context, Result};
use std::path::Path;
use tokio::process::Command;

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

            let output = cmd
                .output()
                .await
                .context("Failed to execute git clone command")?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_output.push_str(&stdout);
            log_output.push_str(&stderr);

            if !output.status.success() {
                anyhow::bail!("Git clone failed: {}", log_output);
            }
        }
        CloneMethod::Shallow => {
            let mut cmd = Command::new("git");
            cmd.args(["clone", "--depth", "1"]);
            if !branch.is_empty() {
                cmd.args(["--single-branch", "-b", branch]);
            }
            cmd.arg(repo_url).arg(target_dir);

            let output = cmd
                .output()
                .await
                .context("Failed to execute shallow git clone")?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_output.push_str(&stdout);
            log_output.push_str(&stderr);

            if !output.status.success() {
                anyhow::bail!("Shallow clone failed: {}", log_output);
            }
        }
        CloneMethod::Blobless => {
            let mut cmd = Command::new("git");
            cmd.args(["clone", "--filter=blob:none"]);
            if !branch.is_empty() {
                cmd.args(["--single-branch", "-b", branch]);
            }
            cmd.arg(repo_url).arg(target_dir);

            let output = cmd
                .output()
                .await
                .context("Failed to execute blobless git clone")?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_output.push_str(&stdout);
            log_output.push_str(&stderr);

            if !output.status.success() {
                anyhow::bail!("Blobless clone failed: {}", log_output);
            }
        }
        CloneMethod::Treeless => {
            let mut cmd = Command::new("git");
            cmd.args(["clone", "--no-checkout", "--filter=tree:0"]);
            if !branch.is_empty() {
                cmd.args(["--single-branch", "-b", branch]);
            }
            cmd.arg(repo_url).arg(target_dir);

            let output = cmd
                .output()
                .await
                .context("Failed to execute treeless git clone")?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            log_output.push_str(&stdout);
            log_output.push_str(&stderr);

            if !output.status.success() {
                anyhow::bail!("Treeless clone failed: {}", log_output);
            }
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

            let clone_output = clone_cmd
                .output()
                .await
                .context("Failed to initialize sparse clone")?;

            log_output.push_str(&String::from_utf8_lossy(&clone_output.stdout));
            log_output.push_str(&String::from_utf8_lossy(&clone_output.stderr));

            if !clone_output.status.success() {
                anyhow::bail!("Failed initial clone for sparse checkout: {}", log_output);
            }

            let repo_path = Path::new(target_dir);

            // Step 2: Set sparse-checkout --no-cone
            let set_output = Command::new("git")
                .current_dir(repo_path)
                .args(["sparse-checkout", "set", "--no-cone"])
                .output()
                .await
                .context("Failed to set sparse-checkout mode")?;

            log_output.push_str(&String::from_utf8_lossy(&set_output.stdout));
            log_output.push_str(&String::from_utf8_lossy(&set_output.stderr));

            // Step 3: Add directory to sparse-checkout
            let add_output = Command::new("git")
                .current_dir(repo_path)
                .args(["sparse-checkout", "add", "!/*", path_to_clone])
                .output()
                .await
                .context("Failed to add sparse-checkout directory")?;

            log_output.push_str(&String::from_utf8_lossy(&add_output.stdout));
            log_output.push_str(&String::from_utf8_lossy(&add_output.stderr));

            // Step 4: Checkout target branch (or HEAD/main)
            let target_branch = if !branch.is_empty() {
                branch.to_string()
            } else {
                "HEAD".to_string()
            };

            let checkout_output = Command::new("git")
                .current_dir(repo_path)
                .args(["checkout", &target_branch])
                .output()
                .await
                .context("Failed to checkout branch during sparse checkout")?;

            log_output.push_str(&String::from_utf8_lossy(&checkout_output.stdout));
            log_output.push_str(&String::from_utf8_lossy(&checkout_output.stderr));

            if !checkout_output.status.success() {
                anyhow::bail!("Sparse checkout failed at checkout step: {}", log_output);
            }
        }
    }

    Ok(log_output)
}
