export type TransferMode = "project" | "session";

export interface CurrentUserCandidate {
  user_id: string;
  source: string;
  line: string;
  mtime: number;
}

export interface AppDiscoveryResult {
  provider: string;
  app_exe?: string | null;
  app_data_dir?: string | null;
  database_path?: string | null;
  log_dir?: string | null;
  workspace_dir: string;
  backup_dir: string;
  current_user_candidates: CurrentUserCandidate[];
  warnings: string[];
}

export interface DatabaseLocation {
  database_path: string;
  app_exe?: string | null;
  log_dir?: string | null;
  backup_dir?: string | null;
}

export interface AccountSummary {
  user_id: string;
  is_current: boolean;
  sessions: number;
  projects: number;
  messages: number;
  latest_at?: number | null;
}

export interface ProjectSummary {
  project_id: string;
  name: string;
  owner_user_id: string;
  path: string;
  active_sessions: number;
  messages: number;
  latest_at?: number | null;
}

export interface ConversationSummary {
  session_id: string;
  title: string;
  project_id: string;
  project_name: string;
  project_path: string;
  owner_user_id: string;
  messages: number;
  updated_at?: number | null;
  deleted_at: number;
  work_mode?: string | null;
}

export interface AnalysisResult {
  location: DatabaseLocation;
  integrity: string;
  accounts: AccountSummary[];
  target_accounts: AccountSummary[];
  projects: ProjectSummary[];
  conversations: ConversationSummary[];
  excluded_deleted_count: number;
  current_user_candidates: CurrentUserCandidate[];
  default_target_user_id?: string | null;
  warnings: string[];
}

export interface TransferSelection {
  database_path: string;
  target_user_id: string;
  mode: TransferMode;
  selected_project_ids: string[];
  selected_session_ids: string[];
  backup_dir?: string | null;
}

export interface TransferAction {
  kind: string;
  project_id?: string | null;
  target_project_id?: string | null;
  session_ids: string[];
  from_user_id?: string | null;
  to_user_id: string;
  description: string;
}

export interface TransferPlan {
  plan_id: string;
  selection: TransferSelection;
  actions: TransferAction[];
  backup_path: string;
  changed_pages: number[];
  excluded_deleted_count: number;
  warnings: string[];
}

export interface ApplyResult {
  plan_id: string;
  backup_path: string;
  changed_pages: number[];
  integrity: string;
  hmac_status: string;
  report_path: string;
  warnings: string[];
}

export interface VerifyResult {
  log_dir?: string | null;
  malformed_count: number;
  success_lines: string[];
  error_lines: string[];
  warnings: string[];
}

export interface RollbackResult {
  restored: boolean;
  backup_id: string;
  restored_files: string[];
  warnings: string[];
}
