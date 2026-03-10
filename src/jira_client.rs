use crate::models::{IssueSnapshot, JiraSettings};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const JIRA_API_PREFIX: &str = "/rest/api/3";
const SEARCH_PAGE_SIZE: i64 = 100;

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
        let response = self
            .http
            .get(self.credentials.endpoint("/myself"))
            .basic_auth(&self.credentials.email, Some(&self.credentials.api_token))
            .header(ACCEPT, "application/json")
            .send()
            .await
            .context("failed to reach jira /myself endpoint")?
            .error_for_status()
            .context("jira /myself request failed")?;

        let myself = response
            .json::<MyselfResponse>()
            .await
            .context("failed to parse jira /myself response")?;

        Ok(myself.display_name)
    }

    pub async fn fetch_assigned_issues(&self, jql: &str) -> Result<Vec<IssueSnapshot>> {
        let mut start_at = 0_i64;
        let mut collected = Vec::new();

        loop {
            let request = SearchRequest {
                jql,
                start_at,
                max_results: SEARCH_PAGE_SIZE,
                fields: vec!["summary", "status", "assignee", "updated"],
            };

            let response = self
                .http
                .post(self.credentials.endpoint("/search"))
                .basic_auth(&self.credentials.email, Some(&self.credentials.api_token))
                .header(ACCEPT, "application/json")
                .header(CONTENT_TYPE, "application/json")
                .json(&request)
                .send()
                .await
                .context("failed to reach jira /search endpoint")?
                .error_for_status()
                .context("jira /search request failed")?;

            let page = response
                .json::<SearchResponse>()
                .await
                .context("failed to parse jira /search response")?;

            let page_size = page.issues.len();
            collected.extend(page.issues.into_iter().map(map_issue_snapshot));

            let processed_count = start_at + page.max_results;
            if processed_count >= page.total || page_size == 0 {
                break;
            }

            start_at += page.max_results;
        }

        Ok(collected)
    }
}

#[derive(Debug, Deserialize)]
struct MyselfResponse {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Serialize)]
struct SearchRequest<'a> {
    jql: &'a str,
    #[serde(rename = "startAt")]
    start_at: i64,
    #[serde(rename = "maxResults")]
    max_results: i64,
    fields: Vec<&'a str>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    total: i64,
    #[serde(rename = "maxResults")]
    max_results: i64,
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
