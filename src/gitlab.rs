//! GitLab REST API client wrapper.
//!
//! Handles project search and recursive tree retrieval via GitLab API v4.

use crate::config::get_gitlab_token;
use crate::types::{sort_file_items, FileItem, FileType, RepoItem};
use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;

/// HTTP client for GitLab v4 API operations.
pub struct GitLabClient {
    client: reqwest::Client,
    token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GlProject {
    id: u64,
    name: String,
    path_with_namespace: String,
    description: Option<String>,
    http_url_to_repo: String,
    default_branch: Option<String>,
    star_count: Option<u32>,
    forks_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GlTreeItem {
    name: String,
    path: String,
    #[serde(rename = "type")]
    item_type: String,
    id: String,
}

impl GitLabClient {
    /// Creates a new GitLabClient, injecting a PRIVATE-TOKEN header if discovered.
    pub fn new() -> Result<Self> {
        let token = get_gitlab_token();
        let mut headers = HeaderMap::new();
        if let Some(ref tok) = token {
            if let Ok(val) = HeaderValue::from_str(tok) {
                headers.insert("PRIVATE-TOKEN", val);
            }
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("gitfm-tui/0.1.0")
            .build()
            .context("Failed to build reqwest client for GitLab")?;

        Ok(Self { client, token })
    }

    /// Checks if a non-empty personal access token is configured.
    #[must_use]
    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    /// Searches for GitLab projects matching the given query string.
    pub async fn search_projects(&self, query: &str) -> Result<Vec<RepoItem>> {
        let url = format!(
            "https://gitlab.com/api/v4/search?scope=projects&search={}",
            urlencoding(query)
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to query GitLab search API")?;

        if !response.status().is_success() {
            anyhow::bail!("GitLab API returned error: {}", response.status());
        }

        let projects: Vec<GlProject> = response
            .json()
            .await
            .context("Failed to parse GitLab search JSON")?;

        let items = projects
            .into_iter()
            .map(|p| RepoItem {
                id: p.id.to_string(),
                name: p.name,
                full_name: p.path_with_namespace,
                description: p.description,
                html_url: p.http_url_to_repo.clone(),
                clone_url: p.http_url_to_repo,
                default_branch: p.default_branch.unwrap_or_else(|| "main".to_string()),
                stars: p.star_count.unwrap_or(0),
                forks: p.forks_count.unwrap_or(0),
                language: None,
                platform: "GitLab".to_string(),
            })
            .collect();

        Ok(items)
    }

    /// Retrieves repository tree items (files and folders) at the specified directory path.
    pub async fn get_tree(&self, project_id: &str, path: &str) -> Result<Vec<FileItem>> {
        let encoded_id = urlencoding(project_id);
        let path_param = if path.is_empty() {
            String::new()
        } else {
            std::format!("&path={}", urlencoding(path))
        };
        let url = std::format!(
            "https://gitlab.com/api/v4/projects/{}/repository/tree?per_page=100{}",
            encoded_id, path_param
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to get GitLab repository tree")?;

        if !response.status().is_success() {
            anyhow::bail!("GitLab API tree error: {}", response.status());
        }

        let tree_items: Vec<GlTreeItem> = response
            .json()
            .await
            .context("Failed to parse GitLab tree JSON")?;

        let mut items = Vec::with_capacity(tree_items.len());
        for item in tree_items {
            let file_type = match item.item_type.as_str() {
                "tree" => FileType::Directory,
                "blob" => FileType::File,
                _ => FileType::File,
            };

            items.push(FileItem {
                name: item.name,
                path: item.path,
                file_type,
                size: None,
                sha: Some(item.id),
                download_url: None,
            });
        }

        // Sort directories first, then alphabetically
        sort_file_items(&mut items);

        Ok(items)
    }
}

/// Encodes query strings or path parameters safely for URL queries.
pub fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlencoding_simple() {
        assert_eq!(urlencoding("hello world"), "hello+world");
        assert_eq!(urlencoding("foo/bar"), "foo%2Fbar");
        assert_eq!(urlencoding("user@domain"), "user%40domain");
    }
}
