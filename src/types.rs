//! Core domain types, enumerations, and data representations for gitFM.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported remote Git hosting platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Platform {
    GitHub,
    GitLab,
}

impl Platform {
    /// Returns the user-facing display name of the platform.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
        }
    }

    /// Toggles between GitHub and GitLab.
    #[must_use]
    pub const fn toggle(self) -> Self {
        match self {
            Self::GitHub => Self::GitLab,
            Self::GitLab => Self::GitHub,
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Unified repository representation across GitHub and GitLab.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoItem {
    pub id: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub clone_url: String,
    pub default_branch: String,
    pub stars: u32,
    pub forks: u32,
    pub language: Option<String>,
    pub platform: String,
}

/// Node kind within a repository tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Directory,
    File,
    Symlink,
    Submodule,
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Directory => write!(f, "Directory"),
            Self::File => write!(f, "File"),
            Self::Symlink => write!(f, "Symlink"),
            Self::Submodule => write!(f, "Submodule"),
        }
    }
}

/// Unified representation of a tree node (file or directory).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    pub name: String,
    pub path: String,
    pub file_type: FileType,
    pub size: Option<u64>,
    pub sha: Option<String>,
    pub download_url: Option<String>,
}

/// Sorts file items placing directories first, then sorting alphabetically case-insensitively.
pub fn sort_file_items(items: &mut [FileItem]) {
    items.sort_by(|a, b| match (a.file_type, b.file_type) {
        (FileType::Directory, FileType::Directory) => {
            a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
        (FileType::Directory, _) => std::cmp::Ordering::Less,
        (_, FileType::Directory) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
}

/// Supported Git clone strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneMethod {
    Normal,
    Shallow,
    Blobless,
    Treeless,
    Sparse,
}

impl CloneMethod {
    /// Returns all available cloning strategies in preferred selection order.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Normal,
            Self::Shallow,
            Self::Blobless,
            Self::Treeless,
            Self::Sparse,
        ]
    }

    /// User-friendly label with git flag indicators.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Normal => "Normal Clone",
            Self::Shallow => "Shallow Clone (--depth 1)",
            Self::Blobless => "Blobless Clone (--filter=blob:none)",
            Self::Treeless => "Treeless Clone (--filter=tree:0)",
            Self::Sparse => "Sparse Checkout (Current Folder)",
        }
    }

    /// Concise description of how this clone strategy works.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::Normal => "Clones the full repository with complete history and all files.",
            Self::Shallow => "Fetches only the latest commit. Minimal download size for quick inspection.",
            Self::Blobless => "Downloads commit and tree history. Blobs are downloaded on-demand.",
            Self::Treeless => "Downloads commit history only. Trees and blobs are fetched on-demand.",
            Self::Sparse => "Clones only the currently selected folder in the monorepo.",
        }
    }
}

impl fmt::Display for CloneMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Current active modal or interaction state of the TUI application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    SearchInput,
    CloneModal,
    HelpModal,
    CloningInProgress,
    CloneFinished,
}

/// Currently focused Miller column pane in the Yazi file-manager view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    RepoList,
    FileList,
    Preview,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_toggle() {
        assert_eq!(Platform::GitHub.toggle(), Platform::GitLab);
        assert_eq!(Platform::GitLab.toggle(), Platform::GitHub);
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(Platform::GitHub.to_string(), "GitHub");
        assert_eq!(Platform::GitLab.to_string(), "GitLab");
    }

    #[test]
    fn test_clone_methods() {
        let all = CloneMethod::all();
        assert_eq!(all.len(), 5);
        assert_eq!(all[0], CloneMethod::Normal);
        assert!(all[1].name().contains("--depth 1"));
        assert_eq!(CloneMethod::Normal.to_string(), "Normal Clone");
    }

    #[test]
    fn test_file_type_display() {
        assert_eq!(FileType::Directory.to_string(), "Directory");
        assert_eq!(FileType::File.to_string(), "File");
    }

    #[test]
    fn test_sort_file_items() {
        let mut items = vec![
            FileItem {
                name: "zoo.rs".to_string(),
                path: "zoo.rs".to_string(),
                file_type: FileType::File,
                size: Some(10),
                sha: None,
                download_url: None,
            },
            FileItem {
                name: "src".to_string(),
                path: "src".to_string(),
                file_type: FileType::Directory,
                size: None,
                sha: None,
                download_url: None,
            },
            FileItem {
                name: "apple.rs".to_string(),
                path: "apple.rs".to_string(),
                file_type: FileType::File,
                size: Some(20),
                sha: None,
                download_url: None,
            },
            FileItem {
                name: "assets".to_string(),
                path: "assets".to_string(),
                file_type: FileType::Directory,
                size: None,
                sha: None,
                download_url: None,
            },
        ];

        sort_file_items(&mut items);

        // Directories first alphabetically, then files alphabetically
        assert_eq!(items[0].name, "assets");
        assert_eq!(items[1].name, "src");
        assert_eq!(items[2].name, "apple.rs");
        assert_eq!(items[3].name, "zoo.rs");
    }
}
