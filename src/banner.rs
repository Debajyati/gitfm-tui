//! Greeting banners loader and randomizer.
//!
//! Loads graphical ASCII font banners from the `graphical fonts` directory,
//! falling back to embedded compile-time banners if the directory is missing.

use std::fs;
use std::path::PathBuf;

const BANNER_3D_BLOCKS: &str = include_str!("../graphical fonts/3d-blocks.txt");
const BANNER_DARK_WITH_SHADOW: &str = include_str!("../graphical fonts/dark-with-shadow.txt");
const BANNER_GHOST: &str = include_str!("../graphical fonts/ghost.txt");

/// Loads all greeting banner strings available on disk or from embedded fallbacks.
#[must_use]
pub fn load_greeting_banners() -> Vec<String> {
    let mut banners = Vec::new();

    // Check candidate directory locations for "graphical fonts"
    let candidate_dirs = [
        PathBuf::from("graphical fonts"),
        PathBuf::from("./graphical fonts"),
    ];

    for dir in &candidate_dirs {
        if dir.is_dir() {
            if let Ok(entries) = fs::read_dir(dir) {
                let mut dir_banners = Vec::new();
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("txt") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            let trimmed = content.trim_matches('\n').to_string();
                            if !trimmed.trim().is_empty() {
                                dir_banners.push(trimmed);
                            }
                        }
                    }
                }
                if !dir_banners.is_empty() {
                    banners = dir_banners;
                    break;
                }
            }
        }
    }

    if banners.is_empty() {
        banners.push(BANNER_3D_BLOCKS.trim_matches('\n').to_string());
        banners.push(BANNER_DARK_WITH_SHADOW.trim_matches('\n').to_string());
        banners.push(BANNER_GHOST.trim_matches('\n').to_string());
    }

    banners
}

/// Selects a pseudo-random banner from the available slice using system timestamp nanos.
#[must_use]
pub fn pick_random_banner(banners: &[String]) -> String {
    if banners.is_empty() {
        return String::new();
    }
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as usize)
        .unwrap_or(0);
    banners[seed % banners.len()].clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_greeting_banners() {
        let banners = load_greeting_banners();
        assert!(banners.len() >= 3, "Should load at least 3 banners");
        for b in banners {
            assert!(!b.is_empty(), "Banner content should not be empty");
        }
    }

    #[test]
    fn test_pick_random_banner() {
        let sample = vec!["Banner1".to_string(), "Banner2".to_string()];
        let picked = pick_random_banner(&sample);
        assert!(picked == "Banner1" || picked == "Banner2");

        let empty: Vec<String> = Vec::new();
        assert_eq!(pick_random_banner(&empty), "");
    }
}
