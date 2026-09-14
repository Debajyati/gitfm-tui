//! Token discovery and configuration loader for GitHub and GitLab.
//!
//! Automatically retrieves personal access tokens from environment variables
//! or gitFM configuration files in the user's home directory (`~/.gitfmrc.json` and `~/.gl.gitfmrc.json`).

use base64::prelude::*;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct StoredTokenFile {
    token: Option<String>,
}

/// Generic helper to discover an API token from a list of environment variable names,
/// falling back to a JSON config file in `~/<filename>`.
fn get_token_from_env_or_file(env_vars: &[&str], config_filename: &str) -> Option<String> {
    for &var in env_vars {
        if let Ok(tok) = std::env::var(var) {
            let trimmed = tok.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    if let Some(home) = dirs::home_dir() {
        let path: PathBuf = home.join(config_filename);
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str::<StoredTokenFile>(&content) {
                if let Some(tok) = parsed.token {
                    return Some(decode_token_if_base64(&tok));
                }
            }
        }
    }

    None
}

/// Discovers GitHub token from `GITHUB_TOKEN`, `GH_TOKEN`, or `~/.gitfmrc.json`.
#[must_use]
pub fn get_github_token() -> Option<String> {
    get_token_from_env_or_file(&["GITHUB_TOKEN", "GH_TOKEN"], ".gitfmrc.json")
}

/// Discovers GitLab token from `GITLAB_TOKEN`, `GL_TOKEN`, or `~/.gl.gitfmrc.json`.
#[must_use]
pub fn get_gitlab_token() -> Option<String> {
    get_token_from_env_or_file(&["GITLAB_TOKEN", "GL_TOKEN"], ".gl.gitfmrc.json")
}

/// Decodes a string if it is valid ASCII/UTF-8 base64, otherwise returns original trimmed string.
pub fn decode_token_if_base64(raw: &str) -> String {
    let trimmed = raw.trim();
    if let Ok(bytes) = BASE64_STANDARD.decode(trimmed) {
        if let Ok(decoded_str) = String::from_utf8(bytes) {
            if !decoded_str.is_empty() && decoded_str.chars().all(|c| !c.is_control()) {
                return decoded_str;
            }
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_token_plain() {
        let raw = "ghp_1234567890abcdef";
        assert_eq!(decode_token_if_base64(raw), "ghp_1234567890abcdef");
    }

    #[test]
    fn test_decode_token_with_whitespace() {
        let raw = "  ghp_1234567890abcdef \n";
        assert_eq!(decode_token_if_base64(raw), "ghp_1234567890abcdef");
    }

    #[test]
    fn test_decode_token_base64() {
        // "my_secret_token_123" in base64 is "bXlfc2VjcmV0X3Rva2VuXzEyMw=="
        let encoded = "bXlfc2VjcmV0X3Rva2VuXzEyMw==";
        assert_eq!(decode_token_if_base64(encoded), "my_secret_token_123");
    }

    #[test]
    fn test_decode_token_binary_base64_falls_back() {
        // Non-printable control characters shouldn't be treated as tokens
        // base64 for bytes [0x00, 0x01, 0x02] is "AAEC"
        let encoded = "AAEC";
        assert_eq!(decode_token_if_base64(encoded), "AAEC");
    }
}
