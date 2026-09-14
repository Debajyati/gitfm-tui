use base64::prelude::*;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct StoredTokenFile {
    token: Option<String>,
}

pub fn get_github_token() -> Option<String> {
    if let Ok(tok) = std::env::var("GITHUB_TOKEN") {
        let trimmed = tok.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    if let Ok(tok) = std::env::var("GH_TOKEN") {
        let trimmed = tok.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }

    if let Some(home) = dirs::home_dir() {
        let path: PathBuf = home.join(".gitfmrc.json");
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

pub fn get_gitlab_token() -> Option<String> {
    if let Ok(tok) = std::env::var("GITLAB_TOKEN") {
        let trimmed = tok.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }
    if let Ok(tok) = std::env::var("GL_TOKEN") {
        let trimmed = tok.trim().to_string();
        if !trimmed.is_empty() {
            return Some(trimmed);
        }
    }

    if let Some(home) = dirs::home_dir() {
        let path: PathBuf = home.join(".gl.gitfmrc.json");
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

fn decode_token_if_base64(raw: &str) -> String {
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
