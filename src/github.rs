use crate::config::get_github_token;
use crate::types::{FileItem, FileType, RepoItem};
use anyhow::{Context, Result};
use base64::prelude::*;
use octocrab::models::repos::Content;
use octocrab::Octocrab;
use std::sync::Arc;

pub struct GitHubClient {
    client: Arc<Octocrab>,
}

impl GitHubClient {
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
                language: r.language.and_then(|v| v.as_str().map(|s| s.to_string())),
                platform: "GitHub".to_string(),
            })
            .collect();

        Ok(repos)
    }

    pub async fn get_contents(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
    ) -> Result<Vec<FileItem>> {
        let repos_handler = self.client.repos(owner, repo);
        let mut handler = repos_handler.get_content();
        if !path.is_empty() {
            handler = handler.path(path);
        }

        let content_items = handler
            .send()
            .await
            .context(format!("Failed to get contents for {}/{} at '{}'", owner, repo, path))?;

        let mut items = Vec::new();
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

        // Sort: directories first, then files alphabetically
        items.sort_by(|a, b| match (a.file_type, b.file_type) {
            (FileType::Directory, FileType::Directory) => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            (FileType::Directory, _) => std::cmp::Ordering::Less,
            (_, FileType::Directory) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        });

        Ok(items)
    }

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
            if let Some(Content { content: Some(encoded), .. }) = Some(first) {
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
