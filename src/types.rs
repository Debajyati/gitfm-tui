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
            Self::Shallow => {
                "Fetches only the latest commit. Minimal download size for quick inspection."
            }
            Self::Blobless => "Downloads commit and tree history. Blobs are downloaded on-demand.",
            Self::Treeless => {
                "Downloads commit history only. Trees and blobs are fetched on-demand."
            }
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

/// An editable text input buffer maintaining cursor position and UTF-8 character navigation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InputBuffer {
    value: String,
    cursor: usize,
}

impl InputBuffer {
    /// Creates a new empty input buffer with cursor at 0.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
        }
    }

    /// Creates an input buffer initialized with content, placing cursor at the end.
    #[must_use]
    pub fn from_str(s: &str) -> Self {
        let cursor = s.chars().count();
        Self {
            value: s.to_string(),
            cursor,
        }
    }

    /// Returns the current text contents.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Returns the cursor position as a character count index (0..=char_count).
    #[must_use]
    pub const fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the number of characters in the input buffer.
    #[must_use]
    pub fn char_len(&self) -> usize {
        self.value.chars().count()
    }

    /// Returns true if the buffer has no characters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Inserts a character at the current cursor position and advances the cursor.
    pub fn insert_char(&mut self, c: char) {
        let byte_offset = self.cursor_byte_offset();
        self.value.insert(byte_offset, c);
        self.cursor += 1;
    }

    /// Deletes the character immediately preceding the cursor (Backspace).
    pub fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let byte_offset = self.cursor_byte_offset();
            self.value.remove(byte_offset);
        }
    }

    /// Deletes the character at the current cursor position (Delete).
    pub fn delete(&mut self) {
        let len = self.char_len();
        if self.cursor < len {
            let byte_offset = self.cursor_byte_offset();
            self.value.remove(byte_offset);
        }
    }

    /// Moves the cursor one character to the left.
    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Moves the cursor one character to the right.
    pub fn move_right(&mut self) {
        let len = self.char_len();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    /// Moves the cursor to the start of the input.
    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    /// Moves the cursor to the end of the input.
    pub fn move_end(&mut self) {
        self.cursor = self.char_len();
    }

    /// Clears the content and resets the cursor to 0.
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    /// Replaces the buffer content and positions the cursor at the end.
    pub fn set(&mut self, s: impl Into<String>) {
        let val = s.into();
        self.cursor = val.chars().count();
        self.value = val;
    }

    /// Converts the current character cursor position into a UTF-8 byte index.
    fn cursor_byte_offset(&self) -> usize {
        self.value
            .char_indices()
            .nth(self.cursor)
            .map_or(self.value.len(), |(idx, _)| idx)
    }
}

impl fmt::Display for InputBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl From<&str> for InputBuffer {
    fn from(s: &str) -> Self {
        Self::from_str(s)
    }
}

impl From<String> for InputBuffer {
    fn from(s: String) -> Self {
        let cursor = s.chars().count();
        Self { value: s, cursor }
    }
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

    #[test]
    fn test_input_buffer_insert_and_navigate() {
        let mut buf = InputBuffer::new();
        assert_eq!(buf.value(), "");
        assert_eq!(buf.cursor(), 0);
        assert!(buf.is_empty());

        buf.insert_char('h');
        buf.insert_char('i');
        assert_eq!(buf.value(), "hi");
        assert_eq!(buf.cursor(), 2);

        buf.move_left();
        assert_eq!(buf.cursor(), 1);
        buf.insert_char('o');
        assert_eq!(buf.value(), "hoi");
        assert_eq!(buf.cursor(), 2);

        buf.move_home();
        assert_eq!(buf.cursor(), 0);
        buf.move_left(); // should clamp at 0
        assert_eq!(buf.cursor(), 0);

        buf.move_end();
        assert_eq!(buf.cursor(), 3);
        buf.move_right(); // should clamp at 3
        assert_eq!(buf.cursor(), 3);
    }

    #[test]
    fn test_input_buffer_backspace_and_delete() {
        let mut buf = InputBuffer::from_str("hello");
        assert_eq!(buf.cursor(), 5);

        buf.backspace();
        assert_eq!(buf.value(), "hell");
        assert_eq!(buf.cursor(), 4);

        buf.move_home();
        buf.backspace(); // nothing to delete at home
        assert_eq!(buf.value(), "hell");
        assert_eq!(buf.cursor(), 0);

        buf.delete(); // deletes 'h'
        assert_eq!(buf.value(), "ell");
        assert_eq!(buf.cursor(), 0);

        buf.move_end();
        buf.delete(); // nothing to delete at end
        assert_eq!(buf.value(), "ell");
    }

    #[test]
    fn test_input_buffer_utf8_handling() {
        let mut buf = InputBuffer::from_str("🦀rust");
        assert_eq!(buf.char_len(), 5);
        assert_eq!(buf.cursor(), 5);

        buf.move_home();
        assert_eq!(buf.cursor(), 0);
        buf.move_right();
        assert_eq!(buf.cursor(), 1);

        // Delete 'r' after crab
        buf.delete();
        assert_eq!(buf.value(), "🦀ust");

        // Backspace crab
        buf.backspace();
        assert_eq!(buf.value(), "ust");
        assert_eq!(buf.cursor(), 0);
    }

    #[test]
    fn test_input_buffer_clear_and_set() {
        let mut buf = InputBuffer::from_str("test");
        buf.clear();
        assert_eq!(buf.value(), "");
        assert_eq!(buf.cursor(), 0);

        buf.set("new_repo");
        assert_eq!(buf.value(), "new_repo");
        assert_eq!(buf.cursor(), 8);
    }
}
