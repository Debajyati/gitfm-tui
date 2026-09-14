use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    GitHub,
    GitLab,
}

impl Platform {
    pub fn name(&self) -> &'static str {
        match self {
            Platform::GitHub => "GitHub",
            Platform::GitLab => "GitLab",
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Directory,
    File,
    Symlink,
    Submodule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileItem {
    pub name: String,
    pub path: String,
    pub file_type: FileType,
    pub size: Option<u64>,
    pub sha: Option<String>,
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneMethod {
    Normal,
    Shallow,
    Blobless,
    Treeless,
    Sparse,
}

impl CloneMethod {
    pub fn all() -> &'static [CloneMethod] {
        &[
            CloneMethod::Normal,
            CloneMethod::Shallow,
            CloneMethod::Blobless,
            CloneMethod::Treeless,
            CloneMethod::Sparse,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            CloneMethod::Normal => "Normal Clone",
            CloneMethod::Shallow => "Shallow Clone (--depth 1)",
            CloneMethod::Blobless => "Blobless Clone (--filter=blob:none)",
            CloneMethod::Treeless => "Treeless Clone (--filter=tree:0)",
            CloneMethod::Sparse => "Sparse Checkout (Current Folder)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            CloneMethod::Normal => "Clones the full repository with complete history and all files.",
            CloneMethod::Shallow => "Fetches only the latest commit. Minimal download size for quick inspection.",
            CloneMethod::Blobless => "Downloads commit and tree history. Blobs are downloaded on-demand.",
            CloneMethod::Treeless => "Downloads commit history only. Trees and blobs are fetched on-demand.",
            CloneMethod::Sparse => "Clones only the currently selected folder in the monorepo.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Normal,
    SearchInput,
    CloneModal,
    HelpModal,
    CloningInProgress,
    CloneFinished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    RepoList,
    FileList,
    Preview,
}
