use crate::git::execute_clone;
use crate::github::GitHubClient;
use crate::gitlab::GitLabClient;
use crate::types::{AppMode, CloneMethod, FileItem, FileType, FocusedPane, Platform, RepoItem};
use anyhow::Result;

pub struct App {
    pub platform: Platform,
    pub mode: AppMode,
    pub focused_pane: FocusedPane,
    pub search_query: String,
    pub search_input: String,

    pub repos: Vec<RepoItem>,
    pub repo_selected_index: usize,

    pub files: Vec<FileItem>,
    pub file_selected_index: usize,
    pub current_path: Vec<String>,

    pub preview_content: Option<String>,
    pub preview_scroll: usize,

    pub is_loading: bool,
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
}

impl App {
    pub fn new() -> Self {
        let gh = GitHubClient::new().ok();
        let gl = GitLabClient::new().ok();

        Self {
            platform: Platform::GitHub,
            mode: AppMode::Normal,
            focused_pane: FocusedPane::RepoList,
            search_query: "gitfm".to_string(),
            search_input: String::new(),

            repos: Vec::new(),
            repo_selected_index: 0,

            files: Vec::new(),
            file_selected_index: 0,
            current_path: Vec::new(),

            preview_content: None,
            preview_scroll: 0,

            is_loading: false,
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
        }
    }

    pub fn selected_repo(&self) -> Option<&RepoItem> {
        self.repos.get(self.repo_selected_index)
    }

    pub fn selected_file(&self) -> Option<&FileItem> {
        self.files.get(self.file_selected_index)
    }

    pub fn current_path_string(&self) -> String {
        self.current_path.join("/")
    }

    pub fn switch_platform(&mut self) {
        self.platform = match self.platform {
            Platform::GitHub => Platform::GitLab,
            Platform::GitLab => Platform::GitHub,
        };
        self.repos.clear();
        self.repo_selected_index = 0;
        self.files.clear();
        self.file_selected_index = 0;
        self.current_path.clear();
        self.preview_content = None;
        self.focused_pane = FocusedPane::RepoList;
        self.status_message = format!("Switched to {}", self.platform.name());
    }

    pub fn select_next_repo(&mut self) {
        if !self.repos.is_empty() && self.repo_selected_index + 1 < self.repos.len() {
            self.repo_selected_index += 1;
        }
    }

    pub fn select_prev_repo(&mut self) {
        if self.repo_selected_index > 0 {
            self.repo_selected_index -= 1;
        }
    }

    pub fn select_next_file(&mut self) {
        if !self.files.is_empty() && self.file_selected_index + 1 < self.files.len() {
            self.file_selected_index += 1;
        }
    }

    pub fn select_prev_file(&mut self) {
        if self.file_selected_index > 0 {
            self.file_selected_index -= 1;
        }
    }

    pub async fn perform_search(&mut self) -> Result<()> {
        let query = self.search_query.trim().to_string();
        if query.is_empty() {
            return Ok(());
        }

        self.is_loading = true;
        self.error_message = None;
        self.status_message = format!("Searching {} for '{}'...", self.platform.name(), query);

        match self.platform {
            Platform::GitHub => {
                if let Some(ref client) = self.github_client {
                    match client.search_repositories(&query).await {
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
                            self.error_message = Some(format!("GitHub search failed: {}", err));
                        }
                    }
                } else {
                    self.error_message = Some("GitHub client not available".to_string());
                }
            }
            Platform::GitLab => {
                if let Some(ref client) = self.gitlab_client {
                    match client.search_projects(&query).await {
                        Ok(items) => {
                            self.repos = items;
                            self.repo_selected_index = 0;
                            self.files.clear();
                            self.file_selected_index = 0;
                            self.current_path.clear();
                            self.preview_content = None;
                            self.status_message = format!("Found {} projects", self.repos.len());
                        }
                        Err(err) => {
                            self.error_message = Some(format!("GitLab search failed: {}", err));
                        }
                    }
                } else {
                    self.error_message = Some("GitLab client not available".to_string());
                }
            }
        }

        self.is_loading = false;
        Ok(())
    }

    pub async fn load_current_folder(&mut self) -> Result<()> {
        let repo = match self.selected_repo() {
            Some(r) => r.clone(),
            None => return Ok(()),
        };

        self.is_loading = true;
        self.error_message = None;
        let path = self.current_path_string();
        self.status_message = format!("Loading {}/{}: /{}...", repo.name, self.platform.name(), path);

        match self.platform {
            Platform::GitHub => {
                if let Some(ref client) = self.github_client {
                    let parts: Vec<&str> = repo.full_name.split('/').collect();
                    if parts.len() == 2 {
                        let (owner, name) = (parts[0], parts[1]);
                        match client.get_contents(owner, name, &path).await {
                            Ok(items) => {
                                self.files = items;
                                self.file_selected_index = 0;
                                self.status_message = format!(
                                    "Loaded {} items in /{}",
                                    self.files.len(),
                                    path
                                );
                            }
                            Err(err) => {
                                self.error_message = Some(format!("Failed to load contents: {}", err));
                            }
                        }
                    }
                }
            }
            Platform::GitLab => {
                if let Some(ref client) = self.gitlab_client {
                    match client.get_tree(&repo.id, &path).await {
                        Ok(items) => {
                            self.files = items;
                            self.file_selected_index = 0;
                            self.status_message = format!(
                                "Loaded {} items in /{}",
                                self.files.len(),
                                path
                            );
                        }
                        Err(err) => {
                            self.error_message = Some(format!("Failed to load GitLab tree: {}", err));
                        }
                    }
                }
            }
        }

        self.is_loading = false;
        Ok(())
    }

    pub async fn load_preview(&mut self) {
        self.preview_content = None;
        self.preview_scroll = 0;

        let repo = match self.selected_repo() {
            Some(r) => r.clone(),
            None => return,
        };

        let file = match self.selected_file() {
            Some(f) => f.clone(),
            None => return,
        };

        if file.file_type == FileType::Directory {
            return;
        }

        if self.platform == Platform::GitHub {
            if let Some(ref client) = self.github_client {
                let parts: Vec<&str> = repo.full_name.split('/').collect();
                if parts.len() == 2 {
                    if let Ok(Some(text)) = client.get_file_preview(parts[0], parts[1], &file.path).await {
                        self.preview_content = Some(text);
                    }
                }
            }
        }
    }

    pub fn prepare_clone_modal(&mut self) {
        let repo = match self.selected_repo() {
            Some(r) => r.clone(),
            None => return,
        };

        let mut target_dir = repo.name.clone();
        let mut default_method = CloneMethod::Normal;

        // If user is inside a folder or selected a folder, default to sparse checkout!
        let subfolder = if let Some(file) = self.selected_file() {
            if file.file_type == FileType::Directory {
                Some(file.path.clone())
            } else if !self.current_path.is_empty() {
                Some(self.current_path_string())
            } else {
                None
            }
        } else if !self.current_path.is_empty() {
            Some(self.current_path_string())
        } else {
            None
        };

        if let Some(folder) = subfolder {
            if !folder.is_empty() {
                default_method = CloneMethod::Sparse;
                if let Some(last_part) = folder.split('/').last() {
                    target_dir = format!("{}-{}", repo.name, last_part);
                }
            }
        }

        self.clone_method = default_method;
        self.clone_dir_input = target_dir;
        self.clone_branch_input = repo.default_branch.clone();
        self.clone_focused_field = 0;
        self.clone_output.clear();
        self.clone_success = false;
        self.mode = AppMode::CloneModal;
    }

    pub async fn start_cloning(&mut self) {
        let repo = match self.selected_repo() {
            Some(r) => r.clone(),
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
            if let Some(file) = self.selected_file() {
                if file.file_type == FileType::Directory {
                    Some(file.path.clone())
                } else if !self.current_path.is_empty() {
                    Some(self.current_path_string())
                } else {
                    None
                }
            } else if !self.current_path.is_empty() {
                Some(self.current_path_string())
            } else {
                None
            }
        } else {
            None
        };

        self.mode = AppMode::CloningInProgress;
        self.status_message = format!("Cloning {} into '{}'...", repo.name, target_dir);

        match execute_clone(
            &repo.clone_url,
            &target_dir,
            &branch,
            method,
            sparse_path.as_deref(),
        )
        .await
        {
            Ok(output) => {
                self.clone_output = format!("Cloning completed successfully!\n\nTarget directory: ./{}\n\n{}", target_dir, output);
                self.clone_success = true;
                self.mode = AppMode::CloneFinished;
                self.status_message = "Cloning succeeded!".to_string();
            }
            Err(err) => {
                self.clone_output = format!("Cloning failed:\n\n{}", err);
                self.clone_success = false;
                self.mode = AppMode::CloneFinished;
                self.status_message = "Cloning failed!".to_string();
            }
        }
    }
}
