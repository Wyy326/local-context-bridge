use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUserCandidate {
    pub user_id: String,
    pub source: String,
    pub line: String,
    pub mtime: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppDiscoveryResult {
    pub provider: String,
    pub app_exe: Option<String>,
    pub app_data_dir: Option<String>,
    pub database_path: Option<String>,
    pub log_dir: Option<String>,
    pub workspace_dir: String,
    pub backup_dir: String,
    pub current_user_candidates: Vec<CurrentUserCandidate>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseLocation {
    pub database_path: String,
    pub app_exe: Option<String>,
    pub log_dir: Option<String>,
    pub backup_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSummary {
    pub user_id: String,
    pub is_current: bool,
    pub sessions: i64,
    pub projects: i64,
    pub messages: i64,
    pub latest_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub project_id: String,
    pub name: String,
    pub owner_user_id: String,
    pub path: String,
    pub active_sessions: i64,
    pub messages: i64,
    pub latest_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSummary {
    pub session_id: String,
    pub title: String,
    pub project_id: String,
    pub project_name: String,
    pub project_path: String,
    pub owner_user_id: String,
    pub messages: i64,
    pub updated_at: Option<i64>,
    pub deleted_at: i64,
    pub work_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub location: DatabaseLocation,
    pub integrity: String,
    pub accounts: Vec<AccountSummary>,
    pub target_accounts: Vec<AccountSummary>,
    pub projects: Vec<ProjectSummary>,
    pub conversations: Vec<ConversationSummary>,
    pub excluded_deleted_count: i64,
    pub current_user_candidates: Vec<CurrentUserCandidate>,
    pub default_target_user_id: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TransferMode {
    Project,
    Session,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSelection {
    pub database_path: String,
    pub target_user_id: String,
    pub mode: TransferMode,
    pub selected_project_ids: Vec<String>,
    pub selected_session_ids: Vec<String>,
    pub backup_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferAction {
    pub kind: String,
    pub project_id: Option<String>,
    pub target_project_id: Option<String>,
    pub session_ids: Vec<String>,
    pub from_user_id: Option<String>,
    pub to_user_id: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferPlan {
    pub plan_id: String,
    pub selection: TransferSelection,
    pub actions: Vec<TransferAction>,
    pub backup_path: String,
    pub changed_pages: Vec<u64>,
    pub excluded_deleted_count: i64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyResult {
    pub plan_id: String,
    pub backup_path: String,
    pub changed_pages: Vec<u64>,
    pub integrity: String,
    pub hmac_status: String,
    pub report_path: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub log_dir: Option<String>,
    pub malformed_count: usize,
    pub success_lines: Vec<String>,
    pub error_lines: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackResult {
    pub restored: bool,
    pub backup_id: String,
    pub restored_files: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PreparedDatabase {
    pub encrypted_source: std::path::PathBuf,
    pub plain_path: std::path::PathBuf,
    pub workspace_dir: std::path::PathBuf,
}
