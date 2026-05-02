// GitHub FS - MCP GitHub Server 实现
//
// VFS 接口暴露 GitHub API 作为文件系统
// 挂载点: /github
//
// 目录结构:
// /github/
// ├── repos/                    # 仓库列表
// │   └── {owner}/{repo}/
// │       ├── info.json          # 仓库信息
// │       ├── issues/            # Issues
// │       │   └── {issue_number}/
// │       │       ├── info.json
// │       │       └── comments/
// │       ├── pulls/             # Pull Requests
// │       │   └── {pr_number}/
// │       ├── branches/          # 分支
// │       ├── commits/           # 提交历史
// │       └── contents/          # 仓库内容 (read-only)
// ├── user/                      # 当前用户
// │   └── info.json
// ├── search/                    # 搜索
// │   └── repos?q={query}
// └── issues/                    # 全局 Issues 搜索
//     └── search?q={query}

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use evif_core::{EvifPlugin, EvifResult, EvifError, FileInfo, WriteFlags};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// GitHub API 响应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepo {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub stargazers_count: i64,
    pub forks_count: i64,
    pub language: Option<String>,
    pub default_branch: String,
    pub private: bool,
    pub updated_at: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub id: i64,
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub html_url: String,
    pub user: GitHubUser,
    pub labels: Vec<GitHubLabel>,
    pub comments: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPullRequest {
    pub id: i64,
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub html_url: String,
    pub user: GitHubUser,
    pub head: GitHubBranch,
    pub base: GitHubBranch,
    pub merged: Option<bool>,
    pub comments: i64,
    pub review_comments: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub id: i64,
    pub avatar_url: String,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLabel {
    pub id: i64,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubBranch {
    pub ref_name: String,
    pub sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCommit {
    pub sha: String,
    pub message: String,
    pub author: GitHubCommitAuthor,
    pub url: String,
    pub files_changed: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCommitAuthor {
    pub name: String,
    pub email: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubContent {
    pub name: String,
    pub path: String,
    pub sha: String,
    pub size: i64,
    pub r#type: String,
    pub content: Option<String>,
    pub encoding: Option<String>,
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubSearchResult {
    pub total_count: i64,
    pub incomplete_results: bool,
    pub items: Vec<GitHubRepo>,
}

/// GitHub FS Plugin
pub struct GitHubFs {
    /// HTTP 客户端
    client: Client,
    /// 认证 Token
    token: Option<String>,
    /// 基础 URL
    base_url: String,
    /// 缓存
    cache: Arc<RwLock<HashMap<String, (Vec<FileInfo>, DateTime<Utc>)>>>,
}

impl GitHubFs {
    /// 创建新的 GitHub FS
    pub fn new(token: Option<String>) -> Self {
        let client = Client::builder()
            .user_agent("EVIF-GitHubFS/1.0")
            .build()
            .unwrap_or_default();

        Self {
            client,
            token,
            base_url: "https://api.github.com".to_string(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建带配置的 GitHub FS
    pub fn with_config(token: String, base_url: Option<String>) -> Self {
        let mut fs = Self::new(Some(token));
        if let Some(url) = base_url {
            fs.base_url = url;
        }
        fs
    }

    /// 获取认证头
    fn auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::ACCEPT,
            "application/vnd.github.v3+json".parse().unwrap(),
        );
        if let Some(token) = &self.token {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", token).parse().unwrap(),
            );
        }
        headers
    }

    /// 列出仓库
    pub async fn list_repos(&self, user: Option<&str>) -> EvifResult<Vec<GitHubRepo>> {
        let url = match user {
            Some(u) => format!("{}/users/{}/repos", self.base_url, u),
            None => format!("{}/user/repos", self.base_url),
        };

        let resp = self.client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(url));
        }

        let repos: Vec<GitHubRepo> = resp.json().await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        Ok(repos)
    }

    /// 获取仓库信息
    pub async fn get_repo(&self, owner: &str, repo: &str) -> EvifResult<GitHubRepo> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);

        let resp = self.client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("{}/{}", owner, repo)));
        }

        let repo: GitHubRepo = resp.json().await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        Ok(repo)
    }

    /// 列出 Issues
    pub async fn list_issues(&self, owner: &str, repo: &str, state: Option<&str>) -> EvifResult<Vec<GitHubIssue>> {
        let state_part = state.unwrap_or("open");
        let url = format!("{}/repos/{}/{}/issues?state={}", self.base_url, owner, repo, state_part);

        let resp = self.client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("{}/{}/issues", owner, repo)));
        }

        let issues: Vec<GitHubIssue> = resp.json().await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        Ok(issues)
    }

    /// 获取 Issue
    pub async fn get_issue(&self, owner: &str, repo: &str, number: i64) -> EvifResult<GitHubIssue> {
        let url = format!("{}/repos/{}/{}/issues/{}", self.base_url, owner, repo, number);

        let resp = self.client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("{}/{}/issues/{}", owner, repo, number)));
        }

        let issue: GitHubIssue = resp.json().await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        Ok(issue)
    }

    /// 列出 Pull Requests
    pub async fn list_pulls(&self, owner: &str, repo: &str, state: Option<&str>) -> EvifResult<Vec<GitHubPullRequest>> {
        let state_part = state.unwrap_or("open");
        let url = format!("{}/repos/{}/{}/pulls?state={}", self.base_url, owner, repo, state_part);

        let resp = self.client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("{}/{}/pulls", owner, repo)));
        }

        let pulls: Vec<GitHubPullRequest> = resp.json().await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        Ok(pulls)
    }

    /// 列出分支
    pub async fn list_branches(&self, owner: &str, repo: &str) -> EvifResult<Vec<GitHubBranch>> {
        let url = format!("{}/repos/{}/{}/branches", self.base_url, owner, repo);

        let resp = self.client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("{}/{}/branches", owner, repo)));
        }

        #[derive(Deserialize)]
        struct Branch {
            name: String,
            commit: BranchCommit,
        }
        #[derive(Deserialize)]
        struct BranchCommit {
            sha: String,
        }

        let branches: Vec<Branch> = resp.json().await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        Ok(branches.into_iter().map(|b| GitHubBranch {
            ref_name: b.name,
            sha: b.commit.sha,
        }).collect())
    }

    /// 获取文件内容
    pub async fn get_content(&self, owner: &str, repo: &str, path: &str, branch: Option<&str>) -> EvifResult<Vec<GitHubContent>> {
        let branch_part = branch.unwrap_or("main");
        let url = format!("{}/repos/{}/{}/contents/{}?ref={}", self.base_url, owner, repo, path, branch_part);

        let resp = self.client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("{}/{}/contents/{}", owner, repo, path)));
        }

        // API 可能返回单个对象或数组
        let content: Vec<GitHubContent> = resp.json().await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        Ok(content)
    }

    /// 搜索仓库
    pub async fn search_repos(&self, query: &str) -> EvifResult<GitHubSearchResult> {
        let url = format!("{}/search/repositories?q={}", self.base_url, urlencoding::encode(query));

        let resp = self.client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound(format!("search:{}", query)));
        }

        let result: GitHubSearchResult = resp.json().await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        Ok(result)
    }

    /// 获取当前用户
    pub async fn get_current_user(&self) -> EvifResult<GitHubUser> {
        let url = format!("{}/user", self.base_url);

        let resp = self.client
            .get(&url)
            .headers(self.auth_headers())
            .send()
            .await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(EvifError::NotFound("user".to_string()));
        }

        let user: GitHubUser = resp.json().await
            .map_err(|e| EvifError::NotFound(e.to_string()))?;

        Ok(user)
    }
}

#[async_trait]
impl EvifPlugin for GitHubFs {
    fn name(&self) -> &str {
        "githubfs"
    }

    async fn readdir(&self, path: &str) -> EvifResult<Vec<FileInfo>> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();

        match parts.as_slice() {
            [] | [""] => {
                // 根目录
                Ok(vec![
                    FileInfo {
                        name: "repos".to_string(),
                        size: 0,
                        mode: 0o755 | 0o40000,
                        modified: Utc::now(),
                        is_dir: true,
                    },
                    FileInfo {
                        name: "user".to_string(),
                        size: 0,
                        mode: 0o755 | 0o40000,
                        modified: Utc::now(),
                        is_dir: true,
                    },
                    FileInfo {
                        name: "search".to_string(),
                        size: 0,
                        mode: 0o755 | 0o40000,
                        modified: Utc::now(),
                        is_dir: true,
                    },
                    FileInfo {
                        name: "issues".to_string(),
                        size: 0,
                        mode: 0o755 | 0o40000,
                        modified: Utc::now(),
                        is_dir: true,
                    },
                ])
            }
            ["repos"] => {
                // 列出用户仓库
                let repos = self.list_repos(None).await?;
                Ok(repos.into_iter().map(|r| FileInfo {
                    name: r.full_name.clone(),
                    size: 0,
                    mode: 0o755 | 0o40000,
                    modified: Utc::now(),
                    is_dir: true,
                }).collect())
            }
            ["repos", owner_repo] => {
                // 列出特定用户的仓库
                let repos = self.list_repos(Some(owner_repo)).await?;
                Ok(repos.into_iter().map(|r| FileInfo {
                    name: r.name.clone(),
                    size: 0,
                    mode: 0o755 | 0o40000,
                    modified: Utc::now(),
                    is_dir: true,
                }).collect())
            }
            ["repos", owner, repo] => {
                // 仓库根目录
                Ok(vec![
                    FileInfo {
                        name: "info.json".to_string(),
                        size: 0,
                        mode: 0o644,
                        modified: Utc::now(),
                        is_dir: false,
                    },
                    FileInfo {
                        name: "issues".to_string(),
                        size: 0,
                        mode: 0o755 | 0o40000,
                        modified: Utc::now(),
                        is_dir: true,
                    },
                    FileInfo {
                        name: "pulls".to_string(),
                        size: 0,
                        mode: 0o755 | 0o40000,
                        modified: Utc::now(),
                        is_dir: true,
                    },
                    FileInfo {
                        name: "branches".to_string(),
                        size: 0,
                        mode: 0o755 | 0o40000,
                        modified: Utc::now(),
                        is_dir: true,
                    },
                    FileInfo {
                        name: "contents".to_string(),
                        size: 0,
                        mode: 0o755 | 0o40000,
                        modified: Utc::now(),
                        is_dir: true,
                    },
                ])
            }
            ["repos", owner, repo, "issues"] => {
                // 列出 Issues
                let issues = self.list_issues(owner, repo, None).await?;
                Ok(issues.into_iter().map(|i| FileInfo {
                    name: format!("{}.json", i.number),
                    size: i.title.len() as u64,
                    mode: 0o644,
                    modified: Utc::now(),
                    is_dir: false,
                }).collect())
            }
            ["repos", owner, repo, "pulls"] => {
                // 列出 Pull Requests
                let pulls = self.list_pulls(owner, repo, None).await?;
                Ok(pulls.into_iter().map(|p| FileInfo {
                    name: format!("{}.json", p.number),
                    size: p.title.len() as u64,
                    mode: 0o644,
                    modified: Utc::now(),
                    is_dir: false,
                }).collect())
            }
            ["repos", owner, repo, "branches"] => {
                // 列出分支
                let branches = self.list_branches(owner, repo).await?;
                Ok(branches.into_iter().map(|b| FileInfo {
                    name: format!("{}.json", b.ref_name),
                    size: b.sha.len() as u64,
                    mode: 0o644,
                    modified: Utc::now(),
                    is_dir: false,
                }).collect())
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn read(&self, path: &str, _offset: u64, _size: u64) -> EvifResult<Vec<u8>> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();

        match parts.as_slice() {
            ["user", "info.json"] => {
                let user = self.get_current_user().await?;
                Ok(serde_json::to_string_pretty(&user)?.into_bytes())
            }
            ["repos", owner, repo, "info.json"] => {
                let repo_info = self.get_repo(owner, repo).await?;
                Ok(serde_json::to_string_pretty(&repo_info)?.into_bytes())
            }
            ["repos", owner, repo, "issues"] => {
                let issues = self.list_issues(owner, repo, None).await?;
                Ok(serde_json::to_string_pretty(&issues)?.into_bytes())
            }
            ["repos", owner, repo, "pulls"] => {
                let pulls = self.list_pulls(owner, repo, None).await?;
                Ok(serde_json::to_string_pretty(&pulls)?.into_bytes())
            }
            ["repos", owner, repo, "branches"] => {
                let branches = self.list_branches(owner, repo).await?;
                Ok(serde_json::to_string_pretty(&branches)?.into_bytes())
            }
            ["repos", owner, repo, "contents", rest @ ..] => {
                let path_str = rest.join("/");
                let content = self.get_content(owner, repo, &path_str, None).await?;
                Ok(serde_json::to_string_pretty(&content)?.into_bytes())
            }
            ["search", rest @ ..] => {
                let query = rest.join("/").trim_start_matches("repos?q=").to_string();
                let result = self.search_repos(&query).await?;
                Ok(serde_json::to_string_pretty(&result)?.into_bytes())
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn stat(&self, path: &str) -> EvifResult<FileInfo> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();

        match parts.as_slice() {
            [] | [""] => Ok(FileInfo {
                name: "github".to_string(),
                size: 0,
                mode: 0o755 | 0o40000,
                modified: Utc::now(),
                is_dir: true,
            }),
            ["repos"] | ["user"] | ["search"] | ["issues"] => Ok(FileInfo {
                name: parts[0].to_string(),
                size: 0,
                mode: 0o755 | 0o40000,
                modified: Utc::now(),
                is_dir: true,
            }),
            ["user", "info.json"] => {
                let _ = self.get_current_user().await?;
                Ok(FileInfo {
                    name: "info.json".to_string(),
                    size: 100,
                    mode: 0o644,
                    modified: Utc::now(),
                    is_dir: false,
                })
            }
            _ => Err(EvifError::NotFound(path.to_string())),
        }
    }

    async fn create(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::PermissionDenied("GitHub repository creation not implemented".to_string()))
    }

    async fn write(&self, _path: &str, _data: Vec<u8>, _offset: i64, _flags: WriteFlags) -> EvifResult<u64> {
        Err(EvifError::PermissionDenied("GitHub write not implemented".to_string()))
    }

    async fn mkdir(&self, _path: &str, _perm: u32) -> EvifResult<()> {
        Err(EvifError::PermissionDenied("GitHub mkdir not implemented".to_string()))
    }

    async fn remove(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::PermissionDenied("GitHub remove not implemented".to_string()))
    }

    async fn rename(&self, _old_path: &str, _new_path: &str) -> EvifResult<()> {
        Err(EvifError::PermissionDenied("GitHub rename not implemented".to_string()))
    }

    async fn remove_all(&self, _path: &str) -> EvifResult<()> {
        Err(EvifError::PermissionDenied("GitHub remove_all not implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_githubfs_creation() {
        let fs = GitHubFs::new(None);
        assert_eq!(fs.name(), "githubfs");
    }

    #[tokio::test]
    async fn test_readdir_root() {
        let fs = GitHubFs::new(None);
        let entries = fs.readdir("/").await.unwrap();
        assert!(entries.len() >= 4);
        assert!(entries.iter().any(|e| e.name == "repos"));
        assert!(entries.iter().any(|e| e.name == "user"));
    }

    #[tokio::test]
    async fn test_readdir_repo_root() {
        let fs = GitHubFs::new(None);
        let entries = fs.readdir("/repos/owner/repo").await.unwrap();
        assert!(entries.iter().any(|e| e.name == "info.json"));
        assert!(entries.iter().any(|e| e.name == "issues"));
        assert!(entries.iter().any(|e| e.name == "pulls"));
    }

    #[tokio::test]
    async fn test_stat_root() {
        let fs = GitHubFs::new(None);
        let info = fs.stat("/").await.unwrap();
        assert!(info.is_dir);
        assert_eq!(info.name, "github");
    }
}
