use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use chrono::Local;
use rand::RngCore;
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    crypto, database,
    error::{CoreError, CoreResult},
    paths, process,
    types::{
        ApplyResult, RollbackResult, TransferAction, TransferMode, TransferPlan, TransferSelection,
    },
};

pub fn preview_transfer(selection: TransferSelection) -> CoreResult<TransferPlan> {
    validate_selection(&selection)?;
    let plan_id = format!("plan-{}", stamp());
    let workspace = paths::workspace_dir()?.join("plans").join(&plan_id);
    fs::create_dir_all(&workspace)?;
    let prepared = crypto::prepare_plain_database(Path::new(&selection.database_path), &workspace)?;
    let patched = workspace.join("database.patched.db");
    fs::copy(&prepared.plain_path, &patched)?;
    let actions = patch_plain_database(&patched, &selection)?;
    let changed_pages = crypto::changed_pages(&prepared.plain_path, &patched)?;
    let backup_path = backup_root(&selection).join(&plan_id);
    let conn = Connection::open(&patched)?;
    let excluded_deleted_count = database::query_excluded_deleted_count(&conn)?;
    let warnings = preview_warnings(&selection, &actions, &changed_pages);
    Ok(TransferPlan {
        plan_id,
        selection,
        actions,
        backup_path: paths::path_string(backup_path),
        changed_pages,
        excluded_deleted_count,
        warnings,
    })
}

pub fn apply_transfer(selection: &TransferSelection, plan_id: &str) -> CoreResult<ApplyResult> {
    validate_selection(selection)?;
    if process::is_trae_running() {
        return Err(CoreError::Message(
            "TRAE SOLO CN is running. Close it before applying a transfer.".to_string(),
        ));
    }

    let live_db = PathBuf::from(&selection.database_path);
    let encrypted = !crypto::is_plain_sqlite(&live_db)?;
    let backup_path = backup_root(selection).join(format!("apply-{}", stamp()));
    let copied = crypto::copy_live_triplet(&live_db, &backup_path)?;

    let workspace = paths::workspace_dir()?.join("apply").join(stamp());
    fs::create_dir_all(&workspace)?;

    if encrypted {
        crypto::checkpoint_wal_into_live(&live_db)?;
    }

    let prepared = crypto::prepare_plain_database(&live_db, &workspace)?;
    let patched = workspace.join("database.patched.db");
    fs::copy(&prepared.plain_path, &patched)?;
    let _actions = patch_plain_database(&patched, selection)?;
    let changed_pages = crypto::changed_pages(&prepared.plain_path, &patched)?;

    if encrypted {
        crypto::apply_encrypted_page_patch(
            &live_db,
            &prepared.plain_path,
            &patched,
            &changed_pages,
        )?;
    } else {
        fs::copy(&patched, &live_db)?;
    }

    let post_plain = backup_path.join("database.post_apply.decrypted.db");
    if encrypted {
        crypto::decrypt_db(&live_db, &post_plain)?;
    } else {
        fs::copy(&live_db, &post_plain)?;
    }
    let conn = Connection::open(&post_plain)?;
    let integrity: String = conn.query_row("pragma integrity_check", [], |row| row.get(0))?;
    let hmac_status = if encrypted {
        let checks = crypto::check_page_hmacs(&live_db, &changed_pages)?;
        if checks.iter().all(|(_, ok)| *ok) {
            "ok".to_string()
        } else {
            "mismatch".to_string()
        }
    } else {
        "plain-sqlite".to_string()
    };

    let report = serde_json::json!({
        "time": Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "plan_id": plan_id,
        "selection": selection,
        "backup_path": paths::path_string(&backup_path),
        "copied_files": copied.iter().map(paths::path_string).collect::<Vec<_>>(),
        "changed_pages": changed_pages.clone(),
        "integrity": integrity.clone(),
        "hmac_status": hmac_status.clone(),
    });
    let report_path = backup_path.join("transfer_report.json");
    fs::write(&report_path, serde_json::to_vec_pretty(&report)?)?;

    Ok(ApplyResult {
        plan_id: plan_id.to_string(),
        backup_path: paths::path_string(backup_path),
        changed_pages,
        integrity,
        hmac_status,
        report_path: paths::path_string(report_path),
        warnings: Vec::new(),
    })
}

pub fn rollback_backup(backup_id: &str) -> CoreResult<RollbackResult> {
    if process::is_trae_running() {
        return Err(CoreError::Message(
            "TRAE SOLO CN is running. Close it before rollback.".to_string(),
        ));
    }
    let backup_dir = PathBuf::from(backup_id);
    if !backup_dir.exists() {
        return Err(CoreError::Message(format!(
            "backup directory not found: {}",
            backup_dir.display()
        )));
    }
    let live_db = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|path| {
            path.join("TRAE SOLO CN")
                .join("ModularData")
                .join("ai-agent")
                .join("database.db")
        })
        .ok_or_else(|| {
            CoreError::Message("APPDATA is not set; cannot locate live database.".to_string())
        })?;
    if !live_db.parent().is_some_and(|parent| parent.exists()) {
        return Err(CoreError::Message(format!(
            "live database directory not found: {}",
            live_db.display()
        )));
    }
    let before_restore = backup_dir
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("rollback-before-{}", stamp()));
    let _ = crypto::copy_live_triplet(&live_db, &before_restore)?;

    let mut restored = Vec::new();
    for suffix in ["", "-wal", "-shm"] {
        let file_name = format!(
            "{}{}",
            live_db
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("database.db"),
            suffix
        );
        let source = backup_dir.join(&file_name);
        let target = live_db.with_file_name(&file_name);
        if source.exists() {
            fs::copy(&source, &target)?;
            restored.push(paths::path_string(target));
        } else if target.exists() && suffix != "" {
            fs::remove_file(&target)?;
        }
    }
    Ok(RollbackResult {
        restored: !restored.is_empty(),
        backup_id: backup_id.to_string(),
        restored_files: restored,
        warnings: vec![format!(
            "回滚前的当前 live 数据库已备份到 {}",
            before_restore.display()
        )],
    })
}

fn patch_plain_database(
    db_path: &Path,
    selection: &TransferSelection,
) -> CoreResult<Vec<TransferAction>> {
    let mut conn = Connection::open(db_path)?;
    let tx = conn.transaction()?;
    let actions = match selection.mode {
        TransferMode::Project => patch_projects(&tx, selection)?,
        TransferMode::Session => patch_sessions(&tx, selection)?,
    };
    let integrity: String = tx.query_row("pragma integrity_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(CoreError::Message(format!(
            "patched database failed integrity_check: {integrity}"
        )));
    }
    tx.commit()?;
    Ok(actions)
}

fn patch_projects(
    conn: &Connection,
    selection: &TransferSelection,
) -> CoreResult<Vec<TransferAction>> {
    let mut actions = Vec::new();
    let mut seen = HashSet::new();
    for project_id in selection
        .selected_project_ids
        .iter()
        .filter(|id| seen.insert((*id).clone()))
    {
        let Some(from_user_id) = database::fetch_project_user(conn, project_id)? else {
            continue;
        };
        if from_user_id == selection.target_user_id {
            continue;
        }
        let sessions = active_sessions_for_project(conn, project_id)?;
        if sessions.is_empty() {
            continue;
        }
        conn.execute(
            "update project set user_id=?, updated_at=max(coalesce(updated_at, 0), ?) where project_id=?",
            params![&selection.target_user_id, now_epoch(), project_id],
        )?;
        actions.push(TransferAction {
            kind: "update_project_owner".to_string(),
            project_id: Some(project_id.clone()),
            target_project_id: Some(project_id.clone()),
            session_ids: sessions,
            from_user_id: Some(from_user_id),
            to_user_id: selection.target_user_id.clone(),
            description: format!("项目级转移: {project_id} -> {}", selection.target_user_id),
        });
    }
    Ok(actions)
}

fn patch_sessions(
    conn: &Connection,
    selection: &TransferSelection,
) -> CoreResult<Vec<TransferAction>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    let mut seen = HashSet::new();
    for session_id in selection
        .selected_session_ids
        .iter()
        .filter(|id| seen.insert((*id).clone()))
    {
        if let Some(project_id) = database::fetch_session_project(conn, session_id)? {
            groups
                .entry(project_id)
                .or_default()
                .push(session_id.clone());
        }
    }

    let mut actions = Vec::new();
    for (source_project_id, selected_sessions) in groups {
        let source = fetch_project(conn, &source_project_id)?;
        if source.user_id == selection.target_user_id {
            continue;
        }
        let all_active = active_sessions_for_project(conn, &source_project_id)?;
        let selected_set = selected_sessions.iter().cloned().collect::<HashSet<_>>();
        let all_set = all_active.iter().cloned().collect::<HashSet<_>>();
        if selected_set == all_set {
            conn.execute(
                "update project set user_id=?, updated_at=max(coalesce(updated_at, 0), ?) where project_id=?",
                params![&selection.target_user_id, now_epoch(), &source_project_id],
            )?;
            actions.push(TransferAction {
                kind: "update_project_owner".to_string(),
                project_id: Some(source_project_id.clone()),
                target_project_id: Some(source_project_id),
                session_ids: selected_sessions,
                from_user_id: Some(source.user_id),
                to_user_id: selection.target_user_id.clone(),
                description: "会话选择覆盖整个项目，降级为项目级转移。".to_string(),
            });
            continue;
        }

        let latest = latest_session_update(conn, &selected_sessions)?;
        let target_project_id =
            match find_reusable_target_project(conn, &source, &selection.target_user_id)? {
                Some(project_id) => project_id,
                None => clone_project(conn, &source, &selection.target_user_id, latest)?,
            };

        for session_id in &selected_sessions {
            conn.execute(
                "update chat_session set project_id=? where session_id=? and deleted_at=0",
                params![&target_project_id, session_id],
            )?;
        }
        if database::table_exists(conn, "session_project")? {
            for session_id in &selected_sessions {
                let changed = conn.execute(
                    "update session_project set project_id=? where session_id=?",
                    params![&target_project_id, session_id],
                )?;
                if changed == 0 {
                    conn.execute(
                        "insert into session_project (project_id, session_id, created_at) values (?, ?, ?)",
                        params![&target_project_id, session_id, now_epoch()],
                    )?;
                }
            }
        }
        actions.push(TransferAction {
            kind: "move_sessions".to_string(),
            project_id: Some(source_project_id),
            target_project_id: Some(target_project_id),
            session_ids: selected_sessions,
            from_user_id: Some(source.user_id),
            to_user_id: selection.target_user_id.clone(),
            description: "会话级转移: 部分选择已重挂到目标用户项目。".to_string(),
        });
    }
    Ok(actions)
}

#[derive(Debug, Clone)]
struct ProjectRow {
    source: String,
    user_id: String,
    name: Option<String>,
    description: Option<String>,
    absolute_path: Option<String>,
    created_at: Option<i64>,
    workspace_status: String,
    transient_fallback_project_id: Option<String>,
    remote_project_id: Option<String>,
    work_mode: Option<String>,
}

fn fetch_project(conn: &Connection, project_id: &str) -> CoreResult<ProjectRow> {
    conn.query_row(
        r#"
        select source, user_id, name, description, absolute_path,
               created_at, workspace_status, transient_fallback_project_id, remote_project_id, work_mode
        from project
        where project_id=?
        "#,
        params![project_id],
        |row| {
            Ok(ProjectRow {
                source: row.get(0)?,
                user_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                absolute_path: row.get(4)?,
                created_at: row.get(5)?,
                workspace_status: row.get(6)?,
                transient_fallback_project_id: row.get(7)?,
                remote_project_id: row.get(8)?,
                work_mode: row.get(9)?,
            })
        },
    )
    .map_err(Into::into)
}

fn active_sessions_for_project(conn: &Connection, project_id: &str) -> CoreResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "select session_id from chat_session where project_id=? and deleted_at=0 order by updated_at desc",
    )?;
    let rows = stmt
        .query_map(params![project_id], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(rows)
}

fn latest_session_update(conn: &Connection, session_ids: &[String]) -> CoreResult<i64> {
    let mut latest = 0i64;
    for session_id in session_ids {
        let value: Option<i64> = conn
            .query_row(
                "select updated_at from chat_session where session_id=?",
                params![session_id],
                |row| row.get(0),
            )
            .optional()?;
        latest = latest.max(value.unwrap_or(0));
    }
    Ok(latest.max(now_epoch()))
}

fn find_reusable_target_project(
    conn: &Connection,
    source: &ProjectRow,
    target_user_id: &str,
) -> CoreResult<Option<String>> {
    let name = source.name.clone().unwrap_or_default();
    let path = source.absolute_path.clone().unwrap_or_default();
    Ok(conn
        .query_row(
            r#"
            select project_id
            from project
            where user_id=?
              and coalesce(name, '')=?
              and coalesce(absolute_path, '')=?
              and deleted_at is null
            order by coalesce(updated_at, 0) desc
            limit 1
            "#,
            params![target_user_id, name, path],
            |row| row.get(0),
        )
        .optional()?)
}

fn clone_project(
    conn: &Connection,
    source: &ProjectRow,
    target_user_id: &str,
    latest_at: i64,
) -> CoreResult<String> {
    let project_id = unique_hex_id(conn, "project_id")?;
    let biz_project_id = unique_biz_project_id(conn, target_user_id)?;
    conn.execute(
        r#"
        insert into project (
            project_id, source, user_id, name, description, absolute_path, biz_project_id,
            created_at, updated_at, deleted_at, workspace_status, transient_fallback_project_id,
            remote_project_id, last_active_at, work_mode
        )
        values (?, ?, ?, ?, ?, ?, ?, ?, ?, null, ?, ?, ?, ?, ?)
        "#,
        params![
            &project_id,
            &source.source,
            target_user_id,
            source.name.as_deref(),
            source.description.as_deref(),
            source.absolute_path.as_deref(),
            &biz_project_id,
            source.created_at.unwrap_or_else(now_epoch),
            latest_at,
            &source.workspace_status,
            source.transient_fallback_project_id.as_deref(),
            source.remote_project_id.as_deref(),
            latest_at,
            source.work_mode.as_deref(),
        ],
    )?;
    Ok(project_id)
}

fn unique_hex_id(conn: &Connection, column: &str) -> CoreResult<String> {
    loop {
        let id = random_hex_24();
        let sql = format!("select count(*) from project where {column}=?");
        let count: i64 = conn.query_row(&sql, params![&id], |row| row.get(0))?;
        if count == 0 {
            return Ok(id);
        }
    }
}

fn unique_biz_project_id(conn: &Connection, target_user_id: &str) -> CoreResult<String> {
    loop {
        let id = random_hex_24();
        let count: i64 = conn.query_row(
            "select count(*) from project where biz_project_id=? and user_id=?",
            params![&id, target_user_id],
            |row| row.get(0),
        )?;
        if count == 0 {
            return Ok(id);
        }
    }
}

fn random_hex_24() -> String {
    let mut bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

fn validate_selection(selection: &TransferSelection) -> CoreResult<()> {
    if selection.database_path.trim().is_empty() {
        return Err(CoreError::Message("database_path is required".to_string()));
    }
    if selection.target_user_id.trim().is_empty() {
        return Err(CoreError::Message("target_user_id is required".to_string()));
    }
    match selection.mode {
        TransferMode::Project if selection.selected_project_ids.is_empty() => Err(
            CoreError::Message("select at least one project".to_string()),
        ),
        TransferMode::Session if selection.selected_session_ids.is_empty() => Err(
            CoreError::Message("select at least one session".to_string()),
        ),
        _ => Ok(()),
    }
}

fn backup_root(selection: &TransferSelection) -> PathBuf {
    selection
        .backup_dir
        .as_ref()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| paths::backup_dir().unwrap_or_else(|_| PathBuf::from("backups")))
}

fn preview_warnings(
    selection: &TransferSelection,
    actions: &[TransferAction],
    changed_pages: &[u64],
) -> Vec<String> {
    let mut warnings = Vec::new();
    if actions.is_empty() {
        warnings.push("没有产生变更；所选内容可能已经属于目标用户。".to_string());
    }
    if selection.mode == TransferMode::Session {
        warnings.push("会话级转移会在部分选择同项目会话时复用或克隆项目。".to_string());
    }
    if changed_pages.len() > 64 {
        warnings.push(format!(
            "变更页数量较多: {}，执行前建议确认备份路径。",
            changed_pages.len()
        ));
    }
    warnings
}

fn now_epoch() -> i64 {
    Local::now().timestamp()
}

fn stamp() -> String {
    Local::now().format("%Y%m%d_%H%M%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::random_hex_24;

    #[test]
    fn generated_project_ids_are_24_lowercase_hex_chars() {
        let id = random_hex_24();
        assert_eq!(id.len(), 24);
        assert!(id
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }
}
