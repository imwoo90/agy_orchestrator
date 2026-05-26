use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProjectInfo {
    pub path: String,
    pub goal: String,
    pub pid: u32,
    pub status: String,
    pub spawned_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Issue {
    pub id: u32,
    pub title: String,
    pub body: String,
    pub status: String, // "open", "in-progress", "resolved", "failed"
    pub created_at: String,
    pub resolved_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ChatMessage {
    pub is_user: bool,
    pub text: String,
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct HealthCheckResult {
    pub target: String,
    pub healthy: bool,
    pub message: String,
    pub checked_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum FeedbackResponse {
    Submitted { title: String, url: String },
    PrefilledUrl { title: String, body: String, url: String },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
pub enum UpgradeProgress {
    Idle,
    Downloading,
    Installing,
    Restarting,
    Success,
    Failed(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChatResponse {
    pub reply: String,
    pub actual_session_id: String,
}
