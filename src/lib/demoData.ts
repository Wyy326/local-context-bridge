import type { AnalysisResult, AppDiscoveryResult, TransferPlan, TransferSelection, VerifyResult } from "./types";

const now = Math.floor(Date.now() / 1000);

export const demoDiscovery: AppDiscoveryResult = {
  provider: "TRAE SOLO CN",
  app_exe: "C:\\Program Files\\TRAE SOLO CN\\TRAE SOLO CN.exe",
  app_data_dir: "C:\\Users\\demo\\AppData\\Roaming\\TRAE SOLO CN",
  database_path: "C:\\Users\\demo\\AppData\\Roaming\\TRAE SOLO CN\\ModularData\\ai-agent\\database.db",
  log_dir: "C:\\Users\\demo\\AppData\\Roaming\\TRAE SOLO CN\\logs",
  workspace_dir: "C:\\Users\\demo\\AppData\\Local\\Local Context Bridge\\workspace",
  backup_dir: "C:\\Users\\demo\\Documents\\Local Context Bridge\\Backups",
  current_user_candidates: [
    {
      user_id: "9000000000000001",
      source: "main.log",
      line: "cached_user_id=Some(\"9000000000000001\")",
      mtime: now,
    },
  ],
  warnings: ["浏览器预览使用 demo 数据；Tauri 桌面运行时会读取本机真实路径。"],
};

export const demoAnalysis: AnalysisResult = {
  location: {
    database_path: demoDiscovery.database_path!,
    app_exe: demoDiscovery.app_exe,
    log_dir: demoDiscovery.log_dir,
    backup_dir: demoDiscovery.backup_dir,
  },
  integrity: "ok",
  default_target_user_id: "9000000000000001",
  current_user_candidates: demoDiscovery.current_user_candidates,
  excluded_deleted_count: 12,
  warnings: demoDiscovery.warnings,
  accounts: [
    {
      user_id: "9000000000000002",
      is_current: false,
      sessions: 5,
      projects: 4,
      messages: 142,
      latest_at: now - 600,
    },
  ],
  target_accounts: [
    {
      user_id: "9000000000000001",
      is_current: true,
      sessions: 0,
      projects: 0,
      messages: 0,
      latest_at: null,
    },
    {
      user_id: "9000000000000002",
      is_current: false,
      sessions: 5,
      projects: 4,
      messages: 142,
      latest_at: now - 600,
    },
  ],
  projects: [
    {
      project_id: "6a4775c961ebfc431c2a4aab",
      name: "Example Workspace",
      owner_user_id: "9000000000000001",
      path: "C:\\Projects\\Example Workspace",
      active_sessions: 2,
      messages: 61,
      latest_at: now - 600,
    },
    {
      project_id: "69fb0575327bb968ca4007c0",
      name: "Research Notes",
      owner_user_id: "9000000000000001",
      path: "C:\\Projects\\Research Notes",
      active_sessions: 2,
      messages: 48,
      latest_at: now - 4000,
    },
  ],
  conversations: [
    {
      session_id: "6a3e5780e738988c4748f974",
      title: "Summarize project structure",
      project_id: "6a4775c961ebfc431c2a4aab",
      project_name: "Example Workspace",
      project_path: "C:\\Projects\\Example Workspace",
      owner_user_id: "9000000000000001",
      messages: 28,
      updated_at: now - 600,
      deleted_at: 0,
      work_mode: "code",
    },
    {
      session_id: "6a47766061ebfc431c2a4ad2",
      title: "Restore context after account switch",
      project_id: "6a4775c961ebfc431c2a4aab",
      project_name: "Example Workspace",
      project_path: "C:\\Projects\\Example Workspace",
      owner_user_id: "9000000000000001",
      messages: 33,
      updated_at: now - 1000,
      deleted_at: 0,
      work_mode: "code",
    },
    {
      session_id: "69fb057a327bb968ca4007c8",
      title: "Build an offline notes prototype",
      project_id: "69fb0575327bb968ca4007c0",
      project_name: "Research Notes",
      project_path: "C:\\Projects\\Research Notes",
      owner_user_id: "9000000000000001",
      messages: 34,
      updated_at: now - 4000,
      deleted_at: 0,
      work_mode: "work",
    },
    {
      session_id: "69fb0585327bb968ca4007d0",
      title: "Validate local API configuration",
      project_id: "69fb0575327bb968ca4007c0",
      project_name: "Research Notes",
      project_path: "C:\\Projects\\Research Notes",
      owner_user_id: "9000000000000002",
      messages: 14,
      updated_at: now - 9000,
      deleted_at: 0,
      work_mode: "code",
    },
  ],
};

export function demoPlan(selection: TransferSelection): TransferPlan {
  const chosen = demoAnalysis.conversations.filter((item) =>
    selection.mode === "project"
      ? selection.selected_project_ids.includes(item.project_id)
      : selection.selected_session_ids.includes(item.session_id),
  );
  return {
    plan_id: `demo-${Date.now()}`,
    selection,
    backup_path: `${demoDiscovery.backup_dir}\\demo-${Date.now()}`,
    changed_pages: [1, 42, 108],
    excluded_deleted_count: demoAnalysis.excluded_deleted_count,
    warnings:
      selection.mode === "session"
        ? ["部分会话选择可能会克隆项目，以避免移动未选中的同项目会话。"]
        : [],
    actions: [
      {
        kind: selection.mode === "project" ? "update_project_owner" : "move_sessions",
        project_id: chosen[0]?.project_id,
        target_project_id: selection.mode === "session" ? "new-demo-project" : chosen[0]?.project_id,
        session_ids: chosen.map((item) => item.session_id),
        from_user_id: chosen[0]?.owner_user_id,
        to_user_id: selection.target_user_id,
        description:
          selection.mode === "project"
            ? `转移 ${selection.selected_project_ids.length} 个项目`
            : `转移 ${selection.selected_session_ids.length} 个会话`,
      },
    ],
  };
}

export const demoVerify: VerifyResult = {
  log_dir: "C:\\Users\\demo\\AppData\\Roaming\\TRAE SOLO CN\\logs\\20260706",
  malformed_count: 0,
  success_lines: ["main.log: [lite][list_chat_sessions] result items_count=11"],
  error_lines: [],
  warnings: [],
};
