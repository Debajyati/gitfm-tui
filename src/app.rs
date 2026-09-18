//! Main application state and business logic coordination.
//!
//! Manages active platform, repositories, directory navigation, file previews,
//! async background task channels, and clone operations.

use crate::git::execute_clone;
use crate::github::GitHubClient;
use crate::gitlab::GitLabClient;
use crate::types::{AppMode, CloneMethod, FileItem, FileType, FocusedPane, Platform, RepoItem};
use anyhow::Result;

/// Braille spinner animation frames (80ms per frame).
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Background asynchronous task results passed back to the main UI loop.
pub enum TaskResult {
    Search {
        platform: Platform,
        query: String,
        result: Result<Vec<RepoItem>, String>,
    },
    FolderContents {
        path: String,
        result: Result<Vec<FileItem>, String>,
    },
    FilePreview {
        path: String,
        result: Result<Option<String>, String>,
    },
    CloneFinished {
        target_dir: String,
        result: Result<String, String>,
    },
}

/// Core application state for gitFM TUI.
pub struct App {
    pub platform: Platform,
    pub mode: AppMode,
    pub focused_pane: FocusedPane,
    pub search_query: String,
    pub search_input: String,
    pub show_dashboard: bool,
    pub greeting_banner: String,

    pub repos: Vec<RepoItem>,
    pub repo_selected_index: usize,

    pub files: Vec<FileItem>,
    pub file_selected_index: usize,
    pub current_path: Vec<String>,

    pub preview_content: Option<String>,
    pub preview_scroll: usize,

    pub is_loading: bool,
    pub is_preview_loading: bool,
    pub spinner_tick: usize,
    pub status_message: String,
    pub error_message: Option<String>,

    // Clone Modal fields
    pub clone_method: CloneMethod,
    pub clone_dir_input: String,
    pub clone_branch_input: String,
    pub clone_focused_field: usize, // 0: Method, 1: Dir, 2: Branch
    pub clone_output: String,
    pub clone_success: bool,

    pub github_client: Option<GitHubClient>,
    pub gitlab_client: Option<GitLabClient>,

    pub tx: tokio::sync::mpsc::UnboundedSender<TaskResult>,
    pub rx: tokio::sync::mpsc::UnboundedReceiver<TaskResult>,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    /// Initializes a new application state with GitHub and GitLab API clients.
    #[must_use]
    pub fn new() -> Self {
        let gh = GitHubClient::new().ok();
        let gl = GitLabClient::new().ok();
        let banners = crate::banner::load_greeting_banners();
        let banner = crate::banner::pick_random_banner(&banners);
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            platform: Platform::GitHub,
            mode: AppMode::Normal,
            focused_pane: FocusedPane::RepoList,
            search_query: String::new(),
            search_input: String::new(),
            show_dashboard: true,
            greeting_banner: banner,

            repos: Vec::new(),
            repo_selected_index: 0,

            files: Vec::new(),
            file_selected_index: 0,
            current_path: Vec::new(),

            preview_content: None,
            preview_scroll: 0,

            is_loading: false,
            is_preview_loading: false,
            spinner_tick: 0,
            status_message: "Press '/' to search, 'Tab' to switch platform, '?' for help".to_string(),
            error_message: None,

            clone_method: CloneMethod::Normal,
            clone_dir_input: String::new(),
            clone_branch_input: String::new(),
            clone_focused_field: 0,
            clone_output: String::new(),
            clone_success: false,

            github_client: gh,
            gitlab_client: gl,

            tx,
            rx,
        }
    }

    /// Returns the current spinner frame character.
    #[must_use]
    pub fn spinner_frame(&self) -> &'static str {
        SPINNER_FRAMES[self.spinner_tick % SPINNER_FRAMES.len()]
    }

    /// Advances the spinner animation frame if any async task is active.
    pub fn tick_spinner(&mut self) {
        if self.is_loading || self.is_preview_loading {
            self.spinner_tick = self.spinner_tick.wrapping_add(1);
        }
    }

    /// Returns a reference to the currently highlighted repository, if any.
    #[must_use]
    pub fn selected_repo(&self) -> Option<&RepoItem> {
        self.repos.get(self.repo_selected_index)
    }

    /// Returns a reference to the currently highlighted file or folder, if any.
    #[must_use]
    pub fn selected_file(&self) -> Option<&FileItem> {
        self.files.get(self.file_selected_index)
    }

    /// Returns the current path formatted as a slash-delimited string (e.g. `src/utils`).
    #[must_use]
    pub fn current_path_string(&self) -> String {
        self.current_path.join("/")
    }

    /// Returns the directory path relevant for sparse cloning (either the currently selected directory item or the current folder path).
    #[must_use]
    pub fn current_selected_directory(&self) -> Option<String> {
        if let Some(file) = self.selected_file() {
            if file.file_type == FileType::Directory {
                return Some(file.path.clone());
            }
        }
        if !self.current_path.is_empty() {
            Some(self.current_path_string())
        } else {
            None
        }
    }

    /// Switches between GitHub and GitLab and resets navigation state.
    pub fn switch_platform(&mut self) {
        self.platform = self.platform.toggle();
        self.repos.clear();
        self.repo_selected_index = 0;
        self.files.clear();
        self.file_selected_index = 0;
        self.current_path.clear();
        self.preview_content = None;
        self.focused_pane = FocusedPane::RepoList;
        self.status_message = format!("Switched to {}", self.platform.name());
    }

    /// Moves repository selection cursor down.
    pub fn select_next_repo(&mut self) {
        if !self.repos.is_empty() && self.repo_selected_index + 1 < self.repos.len() {
            self.repo_selected_index += 1;
        }
    }

    /// Moves repository selection cursor up.
    pub fn select_prev_repo(&mut self) {
        if self.repo_selected_index > 0 {
            self.repo_selected_index -= 1;
        }
    }

    /// Moves file list selection cursor down.
    pub fn select_next_file(&mut self) {
        if !self.files.is_empty() && self.file_selected_index + 1 < self.files.len() {
            self.file_selected_index += 1;
        }
    }

    /// Moves file list selection cursor up.
    pub fn select_prev_file(&mut self) {
        if self.file_selected_index > 0 {
            self.file_selected_index -= 1;
        }
    }

    /// Re-picks a random banner for the dashboard.
    pub fn randomize_banner(&mut self) {
        let banners = crate::banner::load_greeting_banners();
        self.greeting_banner = crate::banner::pick_random_banner(&banners);
    }

    /// Triggers an asynchronous search on the active platform in a background tokio task.
    pub fn trigger_search(&mut self) {
        let query = self.search_query.trim().to_string();
        if query.is_empty() {
            return;
        }

        self.show_dashboard = false;
        self.is_loading = true;
        self.error_message = None;
        self.status_message = format!("Searching {} for '{}'...", self.platform.name(), query);

        let platform = self.platform;
        let tx = self.tx.clone();
        let gh = self.github_client.clone();
        let gl = self.gitlab_client.clone();

        tokio::spawn(async move {
            let result = match platform {
                Platform::GitHub => {
                    if let Some(client) = gh {
                        client.search_repositories(&query).await.map_err(|e| e.to_string())
                    } else {
                        Err("GitHub client not available".to_string())
                    }
                }
                Platform::GitLab => {
                    if let Some(client) = gl {
                        client.search_projects(&query).await.map_err(|e| e.to_string())
                    } else {
                        Err("GitLab client not available".to_string())
                    }
                }
            };

            let _ = tx.send(TaskResult::Search {
                platform,
                query,
                result,
            });
        });
    }

    /// Triggers loading the contents of the current folder in a background tokio task.
    pub fn trigger_load_current_folder(&mut self) {
        let (repo_name, repo_full_name, repo_id) = match self.selected_repo() {
            Some(r) => (r.name.clone(), r.full_name.clone(), r.id.clone()),
            None => return,
        };

        self.is_loading = true;
        self.error_message = None;
        let path = self.current_path_string();
        self.status_message = format!("Loading {}/{}: /{}...", repo_name, self.platform.name(), path);

        let platform = self.platform;
        let tx = self.tx.clone();
        let gh = self.github_client.clone();
        let gl = self.gitlab_client.clone();
        let path_clone = path;

        tokio::spawn(async move {
            let result = match platform {
                Platform::GitHub => {
                    if let Some(client) = gh {
                        let parts: Vec<&str> = repo_full_name.split('/').collect();
                        if parts.len() == 2 {
                            client.get_contents(parts[0], parts[1], &path_clone).await.map_err(|e| e.to_string())
                        } else {
                            Err("Invalid repository full name format".to_string())
                        }
                    } else {
                        Err("GitHub client not available".to_string())
                    }
                }
                Platform::GitLab => {
                    if let Some(client) = gl {
                        client.get_tree(&repo_id, &path_clone).await.map_err(|e| e.to_string())
                    } else {
                        Err("GitLab client not available".to_string())
                    }
                }
            };

            let _ = tx.send(TaskResult::FolderContents {
                path: path_clone,
                result,
            });
        });
    }

    /// Triggers loading preview content for the currently selected file.
    pub fn trigger_load_preview(&mut self) {
        self.preview_content = None;
        self.preview_scroll = 0;

        let repo_full_name = match self.selected_repo() {
            Some(r) => r.full_name.clone(),
            None => return,
        };

        let (file_path, file_type) = match self.selected_file() {
            Some(f) => (f.path.clone(), f.file_type),
            None => return,
        };

        if file_type == FileType::Directory {
            return;
        }

        if self.platform == Platform::GitHub {
            if let Some(client) = self.github_client.clone() {
                let parts: Vec<&str> = repo_full_name.split('/').collect();
                if parts.len() == 2 {
                    let owner = parts[0].to_string();
                    let repo = parts[1].to_string();
                    let path = file_path;
                    let tx = self.tx.clone();
                    self.is_preview_loading = true;

                    tokio::spawn(async move {
                        let result = client.get_file_preview(&owner, &repo, &path).await.map_err(|e| e.to_string());
                        let _ = tx.send(TaskResult::FilePreview {
                            path,
                            result,
                        });
                    });
                }
            }
        }
    }

    /// Prepares parameters and opens the interactive clone modal dialog.
    pub fn prepare_clone_modal(&mut self) {
        let (repo_name, default_branch) = match self.selected_repo() {
            Some(r) => (r.name.clone(), r.default_branch.clone()),
            None => return,
        };

        let mut target_dir = repo_name.clone();
        let mut default_method = CloneMethod::Normal;

        // If user is inside a folder or selected a folder, default to sparse checkout!
        let subfolder = self.current_selected_directory();
        if let Some(folder) = subfolder {
            if !folder.is_empty() {
                default_method = CloneMethod::Sparse;
                if let Some(last_part) = folder.split('/').next_back() {
                    target_dir = format!("{}-{}", repo_name, last_part);
                }
            }
        }

        self.clone_method = default_method;
        self.clone_dir_input = target_dir;
        self.clone_branch_input = default_branch;
        self.clone_focused_field = 0;
        self.clone_output.clear();
        self.clone_success = false;
        self.mode = AppMode::CloneModal;
    }

    /// Triggers git clone execution in the background based on configured options.
    pub fn trigger_start_cloning(&mut self) {
        let (repo_name, clone_url) = match self.selected_repo() {
            Some(r) => (r.name.clone(), r.clone_url.clone()),
            None => return,
        };

        let target_dir = self.clone_dir_input.trim().to_string();
        if target_dir.is_empty() {
            self.error_message = Some("Target directory cannot be empty".to_string());
            return;
        }

        let branch = self.clone_branch_input.trim().to_string();
        let method = self.clone_method;

        let sparse_path = if method == CloneMethod::Sparse {
            self.current_selected_directory()
        } else {
            None
        };

        self.mode = AppMode::CloningInProgress;
        self.is_loading = true;
        self.status_message = format!("Cloning {} into '{}'...", repo_name, target_dir);

        let tx = self.tx.clone();
        let target_dir_clone = target_dir;

        tokio::spawn(async move {
            let result = execute_clone(
                &clone_url,
                &target_dir_clone,
                &branch,
                method,
                sparse_path.as_deref(),
            )
            .await
            .map_err(|e| e.to_string());

            let _ = tx.send(TaskResult::CloneFinished {
                target_dir: target_dir_clone,
                result,
            });
        });
    }

    /// Processes an incoming asynchronous background task result and updates UI state.
    pub fn handle_task_result(&mut self, task_result: TaskResult) {
        match task_result {
            TaskResult::Search { platform, query, result } => {
                if self.platform == platform && self.search_query == query {
                    self.is_loading = false;
                    match result {
                        Ok(items) => {
                            self.repos = items;
                            self.repo_selected_index = 0;
                            self.files.clear();
                            self.file_selected_index = 0;
                            self.current_path.clear();
                            self.preview_content = None;
                            self.status_message = format!("Found {} repositories", self.repos.len());
                        }
                        Err(err) => {
                            self.error_message = Some(format!("{} search failed: {}", platform.name(), err));
                        }
                    }
                }
            }
            TaskResult::FolderContents { path, result } => {
                if self.current_path_string() == path {
                    self.is_loading = false;
                    match result {
                        Ok(items) => {
                            self.files = items;
                            self.file_selected_index = 0;
                            self.status_message = format!("Loaded {} items in /{}", self.files.len(), path);
                            // If first item is a file, trigger preview
                            self.trigger_load_preview();
                        }
                        Err(err) => {
                            self.error_message = Some(format!("Failed to load contents: {}", err));
                        }
                    }
                }
            }
            TaskResult::FilePreview { path, result } => {
                self.is_preview_loading = false;
                if let Some(file) = self.selected_file() {
                    if file.path == path {
                        if let Ok(Some(text)) = result {
                            self.preview_content = Some(text);
                        }
                    }
                }
            }
            TaskResult::CloneFinished { target_dir, result } => {
                self.is_loading = false;
                self.mode = AppMode::CloneFinished;
                match result {
                    Ok(output) => {
                        self.clone_output = format!(
                            "Cloning completed successfully!\n\nTarget directory: ./{}\n\n{}",
                            target_dir, output
                        );
                        self.clone_success = true;
                        self.status_message = "Cloning succeeded!".to_string();
                    }
                    Err(err) => {
                        self.clone_output = format!("Cloning failed:\n\n{}", err);
                        self.clone_success = false;
                        self.status_message = "Cloning failed!".to_string();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_app_initial_state() {
        let app = App::default();
        assert_eq!(app.platform, Platform::GitHub);
        assert_eq!(app.mode, AppMode::Normal);
        assert_eq!(app.focused_pane, FocusedPane::RepoList);
        assert!(app.repos.is_empty());
        assert!(app.files.is_empty());
        assert_eq!(app.current_path_string(), "");
        assert!(app.show_dashboard);
        assert!(!app.greeting_banner.is_empty());
        assert_eq!(app.spinner_frame(), "⠋");
    }

    #[tokio::test]
    async fn test_spinner_rotation() {
        let mut app = App::default();
        assert_eq!(app.spinner_frame(), "⠋");
        app.is_loading = true;
        app.tick_spinner();
        assert_eq!(app.spinner_frame(), "⠙");
        for _ in 0..8 {
            app.tick_spinner();
        }
        assert_eq!(app.spinner_frame(), "⠏");
        app.tick_spinner();
        assert_eq!(app.spinner_frame(), "⠋");
    }

    #[tokio::test]
    async fn test_handle_task_result_search() {
        let mut app = App {
            search_query: "ratatui".to_string(),
            is_loading: true,
            ..Default::default()
        };

        let mock_repos = vec![RepoItem {
            id: "123".to_string(),
            name: "ratatui".to_string(),
            full_name: "ratatui-org/ratatui".to_string(),
            description: Some("Rust library for cooking up terminal user interfaces".to_string()),
            html_url: "https://github.com/ratatui-org/ratatui".to_string(),
            clone_url: "https://github.com/ratatui-org/ratatui.git".to_string(),
            default_branch: "main".to_string(),
            stars: 10000,
            forks: 500,
            language: Some("Rust".to_string()),
            platform: "GitHub".to_string(),
        }];

        app.handle_task_result(TaskResult::Search {
            platform: Platform::GitHub,
            query: "ratatui".to_string(),
            result: Ok(mock_repos),
        });

        assert!(!app.is_loading);
        assert_eq!(app.repos.len(), 1);
        assert_eq!(app.repos[0].name, "ratatui");
        assert!(app.status_message.contains("Found 1 repositories"));
    }

    #[tokio::test]
    async fn test_app_path_string_and_navigation() {
        let mut app = App::default();
        app.current_path.push("src".to_string());
        app.current_path.push("utils".to_string());
        assert_eq!(app.current_path_string(), "src/utils");

        app.current_path.pop();
        assert_eq!(app.current_path_string(), "src");
    }

    #[tokio::test]
    async fn test_app_current_selected_directory() {
        let mut app = App::default();
        // Case 1: Empty state -> None
        assert_eq!(app.current_selected_directory(), None);

        // Case 2: Deep in path -> returns current path string
        app.current_path.push("packages".to_string());
        assert_eq!(app.current_selected_directory(), Some("packages".to_string()));

        // Case 3: Highlighting a directory file item -> returns directory path
        app.files.push(FileItem {
            name: "core".to_string(),
            path: "packages/core".to_string(),
            file_type: FileType::Directory,
            size: None,
            sha: None,
            download_url: None,
        });
        assert_eq!(app.current_selected_directory(), Some("packages/core".to_string()));
    }

    #[tokio::test]
    async fn test_app_repo_selection_bounds() {
        let mut app = App::default();
        app.repos.push(RepoItem {
            id: "1".to_string(),
            name: "repo1".to_string(),
            full_name: "owner/repo1".to_string(),
            description: None,
            html_url: "".to_string(),
            clone_url: "".to_string(),
            default_branch: "main".to_string(),
            stars: 10,
            forks: 2,
            language: None,
            platform: "GitHub".to_string(),
        });
        app.repos.push(RepoItem {
            id: "2".to_string(),
            name: "repo2".to_string(),
            full_name: "owner/repo2".to_string(),
            description: None,
            html_url: "".to_string(),
            clone_url: "".to_string(),
            default_branch: "main".to_string(),
            stars: 20,
            forks: 5,
            language: None,
            platform: "GitHub".to_string(),
        });

        assert_eq!(app.repo_selected_index, 0);
        app.select_next_repo();
        assert_eq!(app.repo_selected_index, 1);
        // Exceeding bounds should clamp
        app.select_next_repo();
        assert_eq!(app.repo_selected_index, 1);

        app.select_prev_repo();
        assert_eq!(app.repo_selected_index, 0);
        // Underflowing should clamp
        app.select_prev_repo();
        assert_eq!(app.repo_selected_index, 0);
    }
}
