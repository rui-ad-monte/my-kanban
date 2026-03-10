use crate::models::{IssueSnapshot, JiraSettings};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use reqwest::header::ACCEPT;
use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const JIRA_API_PREFIX: &str = "/rest/api/3";
const SEARCH_PAGE_SIZE: i64 = 100;
const SEARCH_FIELDS: [&str; 4] = ["summary", "status", "assignee", "updated"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JiraTransition {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct JiraCredentials {
    pub base_url: String,
    pub email: String,
    pub api_token: String,
}

impl JiraCredentials {
    pub fn from_settings(settings: &JiraSettings, api_token: String) -> Result<Self> {
        let base_url = settings.normalized_base_url();
        let email = settings.normalized_email();
        let token = api_token.trim().to_string();

        if base_url.is_empty() {
            return Err(anyhow!("Jira base URL is required"));
        }
        if email.is_empty() {
            return Err(anyhow!("Jira email is required"));
        }
        if token.is_empty() {
            return Err(anyhow!("Jira API token is required"));
        }

        Ok(Self {
            base_url,
            email,
            api_token: token,
        })
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{JIRA_API_PREFIX}{path}", self.base_url)
    }
}

pub struct JiraClient {
    http: Client,
    credentials: JiraCredentials,
}

impl JiraClient {
    pub fn new(credentials: JiraCredentials) -> Result<Self> {
        let http = Client::builder()
            .build()
            .context("failed to build jira http client")?;
        Ok(Self { http, credentials })
    }

    pub async fn test_connection(&self) -> Result<String> {
        let myself = self
            .execute_json::<MyselfResponse>(
                self.http
                    .get(self.credentials.endpoint("/myself"))
                    .basic_auth(&self.credentials.email, Some(&self.credentials.api_token))
                    .header(ACCEPT, "application/json"),
                "/myself",
            )
            .await
            .context("jira /myself request failed")?;

        Ok(myself.display_name)
    }

    pub async fn fetch_assigned_issues(&self, jql: &str) -> Result<Vec<IssueSnapshot>> {
        let mut next_page_token: Option<String> = None;
        let mut collected = Vec::new();

        loop {
            let request = SearchRequest {
                jql: jql.to_string(),
                max_results: SEARCH_PAGE_SIZE,
                fields: SEARCH_FIELDS.into_iter().map(str::to_string).collect(),
                next_page_token: next_page_token.clone(),
            };

            let page = self
                .execute_json::<SearchResponse>(
                    self.http
                        .post(self.credentials.endpoint("/search/jql"))
                        .basic_auth(&self.credentials.email, Some(&self.credentials.api_token))
                        .header(ACCEPT, "application/json")
                        .json(&request),
                    "/search/jql",
                )
                .await
                .context("jira /search/jql request failed")?;

            let page_size = page.issues.len();
            collected.extend(page.issues.into_iter().map(map_issue_snapshot));

            if page_size == 0 {
                break;
            }

            if let Some(token) = page.next_page_token {
                if token.trim().is_empty() {
                    break;
                }
                next_page_token = Some(token);
                continue;
            }

            if let Some(is_last) = page.is_last {
                if is_last || page_size == 0 {
                    break;
                }
            }

            break;
        }

        Ok(collected)
    }

    pub async fn fetch_issue_transitions(&self, issue_key: &str) -> Result<Vec<JiraTransition>> {
        let transitions = self
            .execute_json::<TransitionsResponse>(
                self.http
                    .get(
                        self.credentials
                            .endpoint(&format!("/issue/{issue_key}/transitions")),
                    )
                    .basic_auth(&self.credentials.email, Some(&self.credentials.api_token))
                    .header(ACCEPT, "application/json"),
                &format!("/issue/{issue_key}/transitions"),
            )
            .await
            .with_context(|| format!("jira transitions request failed for {issue_key}"))?;

        Ok(transitions
            .transitions
            .into_iter()
            .map(|transition| JiraTransition {
                id: transition.id,
                name: transition.name,
            })
            .collect())
    }

    pub async fn transition_issue(&self, issue_key: &str, transition_id: &str) -> Result<()> {
        let request = TransitionIssueRequest {
            transition: TransitionRef {
                id: transition_id.to_string(),
            },
        };

        self.execute_no_content(
            self.http
                .post(
                    self.credentials
                        .endpoint(&format!("/issue/{issue_key}/transitions")),
                )
                .basic_auth(&self.credentials.email, Some(&self.credentials.api_token))
                .header(ACCEPT, "application/json")
                .json(&request),
            &format!("/issue/{issue_key}/transitions"),
        )
        .await
    }

    pub async fn add_comment(&self, issue_key: &str, comment: &str) -> Result<()> {
        let request = AddCommentRequest {
            body: CommentDocument::from_text(comment),
        };

        self.execute_no_content(
            self.http
                .post(
                    self.credentials
                        .endpoint(&format!("/issue/{issue_key}/comment")),
                )
                .basic_auth(&self.credentials.email, Some(&self.credentials.api_token))
                .header(ACCEPT, "application/json")
                .json(&request),
            &format!("/issue/{issue_key}/comment"),
        )
        .await
    }

    async fn execute_json<T: DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
        operation: &str,
    ) -> Result<T> {
        let response = request
            .send()
            .await
            .with_context(|| format!("failed to reach jira {operation} endpoint"))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unable to read response body>".to_string());
            return Err(anyhow!(
                "jira {operation} request failed (HTTP {}): {}",
                status.as_u16(),
                compact_error_body(&body)
            ));
        }

        response
            .json::<T>()
            .await
            .with_context(|| format!("failed to parse jira {operation} response"))
    }

    async fn execute_no_content(
        &self,
        request: reqwest::RequestBuilder,
        operation: &str,
    ) -> Result<()> {
        let response = request
            .send()
            .await
            .with_context(|| format!("failed to reach jira {operation} endpoint"))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "<unable to read response body>".to_string());
            return Err(anyhow!(
                "jira {operation} request failed (HTTP {}): {}",
                status.as_u16(),
                compact_error_body(&body)
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct MyselfResponse {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Serialize)]
struct SearchRequest {
    jql: String,
    #[serde(rename = "maxResults")]
    max_results: i64,
    fields: Vec<String>,
    #[serde(rename = "nextPageToken")]
    #[serde(skip_serializing_if = "Option::is_none")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
    #[serde(rename = "isLast")]
    is_last: Option<bool>,
    #[serde(default)]
    issues: Vec<SearchIssue>,
}

#[derive(Debug, Deserialize)]
struct SearchIssue {
    key: String,
    fields: SearchIssueFields,
}

#[derive(Debug, Deserialize)]
struct SearchIssueFields {
    summary: Option<String>,
    status: Option<SearchIssueStatus>,
    assignee: Option<SearchIssueAssignee>,
    updated: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchIssueStatus {
    name: String,
}

#[derive(Debug, Deserialize)]
struct SearchIssueAssignee {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Serialize)]
struct TransitionIssueRequest {
    transition: TransitionRef,
}

#[derive(Debug, Serialize)]
struct TransitionRef {
    id: String,
}

#[derive(Debug, Serialize)]
struct AddCommentRequest {
    body: CommentDocument,
}

#[derive(Debug, Serialize)]
struct CommentDocument {
    #[serde(rename = "type")]
    kind: &'static str,
    version: u8,
    content: Vec<CommentParagraph>,
}

impl CommentDocument {
    fn from_text(text: &str) -> Self {
        Self {
            kind: "doc",
            version: 1,
            content: vec![CommentParagraph {
                kind: "paragraph",
                content: vec![CommentText {
                    kind: "text",
                    text: text.to_string(),
                }],
            }],
        }
    }
}

#[derive(Debug, Serialize)]
struct CommentParagraph {
    #[serde(rename = "type")]
    kind: &'static str,
    content: Vec<CommentText>,
}

#[derive(Debug, Serialize)]
struct CommentText {
    #[serde(rename = "type")]
    kind: &'static str,
    text: String,
}

#[derive(Debug, Deserialize)]
struct TransitionsResponse {
    transitions: Vec<TransitionItem>,
}

#[derive(Debug, Deserialize)]
struct TransitionItem {
    id: String,
    name: String,
}

fn map_issue_snapshot(issue: SearchIssue) -> IssueSnapshot {
    IssueSnapshot {
        issue_key: issue.key,
        summary: issue
            .fields
            .summary
            .unwrap_or_else(|| "(No summary)".to_string()),
        assignee: issue
            .fields
            .assignee
            .map(|assignee| assignee.display_name)
            .unwrap_or_else(|| "Unassigned".to_string()),
        jira_status: issue
            .fields
            .status
            .map(|status| status.name)
            .unwrap_or_else(|| "Unknown".to_string()),
        updated_at: issue
            .fields
            .updated
            .unwrap_or_else(|| Utc::now().to_rfc3339()),
    }
}

fn compact_error_body(body: &str) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        return "<empty response body>".to_string();
    }

    const MAX_LEN: usize = 280;
    if compact.len() <= MAX_LEN {
        compact
    } else {
        format!("{}...", &compact[..MAX_LEN])
    }
}
