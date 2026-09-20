//! GitHub API client wrapper leveraging Octocrab.
//!
//! Provides repository search, remote directory tree browsing, and file preview retrieval.

use crate::config::get_github_token;
use crate::types::{sort_file_items, FileItem, FileType, RepoItem};
use anyhow::{Context, Result};
use base64::prelude::*;
use octocrab::Octocrab;
use std::sync::Arc;

/// Thread-safe client for communicating with GitHub REST API via Octocrab.
#[derive(Clone)]
pub struct GitHubClient {
    client: Arc<Octocrab>,
}

impl GitHubClient {
    /// Creates a new GitHubClient, using personal access token if available.
    pub fn new() -> Result<Self> {
        let mut builder = Octocrab::builder();
        if let Some(token) = get_github_token() {
            builder = builder.personal_token(token);
        }
        let client = builder
            .build()
            .context("Failed to build Octocrab GitHub client")?;
        Ok(Self {
            client: Arc::new(client),
        })
    }

    /// Searches GitHub repositories matching the given query string.
    pub async fn search_repositories(&self, query: &str) -> Result<Vec<RepoItem>> {
        let page = self
            .client
            .search()
            .repositories(query)
            .per_page(40)
            .send()
            .await
            .context("Failed to search GitHub repositories")?;

        let repos = page
            .items
            .into_iter()
            .map(|r| RepoItem {
                id: r.id.to_string(),
                name: r.name,
                full_name: r.full_name.unwrap_or_default(),
                description: r.description,
                html_url: r.html_url.map(|u| u.to_string()).unwrap_or_default(),
                clone_url: r
                    .clone_url
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| format!("https://github.com/{}.git", r.id)),
                default_branch: r.default_branch.unwrap_or_else(|| "main".to_string()),
                stars: r.stargazers_count.unwrap_or(0),
                forks: r.forks_count.unwrap_or(0),
                language: r.language.and_then(|v| v.as_str().map(ToString::to_string)),
                platform: "GitHub".to_string(),
            })
            .collect();

        Ok(repos)
    }

    /// Fetches files and folders in a repository at the specified directory path.
    pub async fn get_contents(&self, owner: &str, repo: &str, path: &str) -> Result<Vec<FileItem>> {
        let repos_handler = self.client.repos(owner, repo);
        let mut handler = repos_handler.get_content();
        if !path.is_empty() {
            handler = handler.path(path);
        }

        let content_items = handler.send().await.with_context(|| {
            format!(
                "Failed to get contents for {}/{} at '{}'",
                owner, repo, path
            )
        })?;

        let mut items = Vec::with_capacity(content_items.items.len());
        for item in content_items.items {
            let file_type = match item.r#type.as_str() {
                "dir" => FileType::Directory,
                "file" => FileType::File,
                "symlink" => FileType::Symlink,
                "submodule" => FileType::Submodule,
                _ => FileType::File,
            };

            items.push(FileItem {
                name: item.name,
                path: item.path,
                file_type,
                size: Some(item.size as u64),
                sha: Some(item.sha),
                download_url: item.download_url,
            });
        }

        // Sort directories first, then alphabetically
        sort_file_items(&mut items);

        Ok(items)
    }

    /// Retrieves file preview content decoded from base64.
    pub async fn get_file_preview(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
    ) -> Result<Option<String>> {
        let content_items = self
            .client
            .repos(owner, repo)
            .get_content()
            .path(path)
            .send()
            .await?;

        if let Some(first) = content_items.items.into_iter().next() {
            if let Some(encoded) = first.content {
                let clean_b64: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();
                if let Ok(bytes) = BASE64_STANDARD.decode(&clean_b64) {
                    if let Ok(text) = String::from_utf8(bytes) {
                        return Ok(Some(text));
                    }
                }
            }
        }
        Ok(None)
    }
}
