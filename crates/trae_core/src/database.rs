use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    crypto,
    error::{CoreError, CoreResult},
    paths,
    types::{
        AccountSummary, AnalysisResult, ConversationSummary, DatabaseLocation, PreparedDatabase,
        ProjectSummary,
    },
};

pub fn analyze_database(location: DatabaseLocation) -> CoreResult<AnalysisResult> {
    let database_path = PathBuf::from(&location.database_path);
    if !database_path.exists() {
        return Err(CoreError::Message(format!(
            "database not found: {}",
            database_path.display()
        )));
    }
    let workspace = paths::workspace_dir()?.join("analysis");
    let prepared = crypto::prepare_plain_database(&database_path, &workspace)?;
    analyze_plain_database(&prepared, location)
}

pub fn analyze_plain_database(
    prepared: &PreparedDatabase,
    location: DatabaseLocation,
) -> CoreResult<AnalysisResult> {
    let conn = Connection::open(&prepared.plain_path)?;
    let integrity: String = conn.query_row("pragma integrity_check", [], |row| row.get(0))?;
    let candidates =
        paths::detect_current_user_candidates(location.log_dir.as_deref().map(Path::new))?;
    let default_target_user_id = candidates
        .first()
        .map(|candidate| candidate.user_id.clone());
    let accounts = query_accounts(&conn, default_target_user_id.as_deref())?;
    let target_accounts = target_accounts(accounts.clone(), default_target_user_id.as_deref());
    let projects = query_projects(&conn)?;
    let conversations = query_conversations(&conn)?;
    let excluded_deleted_count = query_excluded_deleted_count(&conn)?;
    let mut warnings = Vec::new();
    if integrity != "ok" {
        warnings.push(format!("SQLite integrity_check returned {integrity}"));
    }
    if default_target_user_id.is_none() {
        warnings.push("未能从日志识别当前登录用户，需要用户手动选择目标用户。".to_string());
    }
    Ok(AnalysisResult {
        location,
        integrity,
        accounts,
        target_accounts,
        projects,
        conversations,
        excluded_deleted_count,
        current_user_candidates: candidates,
        default_target_user_id,
        warnings,
    })
}

pub fn query_accounts(
    conn: &Connection,
    current_user_id: Option<&str>,
) -> CoreResult<Vec<AccountSummary>> {
    let mut stmt = conn.prepare(
        r#"
        select p.user_id,
               count(cs.session_id) as sessions,
               count(distinct p.project_id) as projects,
               coalesce(sum((select count(*) from chat_message m where m.session_id=cs.session_id)), 0) as messages,
               max(cs.updated_at) as latest_at
        from chat_session cs
        join project p on p.project_id=cs.project_id
        where cs.deleted_at=0
        group by p.user_id
        order by latest_at desc
        "#,
    )?;
    let rows = stmt
        .query_map([], |row| {
            let user_id: String = row.get(0)?;
            Ok(AccountSummary {
                is_current: current_user_id.is_some_and(|current| current == user_id),
                user_id,
                sessions: row.get(1)?,
                projects: row.get(2)?,
                messages: row.get(3)?,
                latest_at: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn target_accounts(
    mut accounts: Vec<AccountSummary>,
    current_user_id: Option<&str>,
) -> Vec<AccountSummary> {
    let Some(current_user_id) = current_user_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return accounts;
    };

    let mut current_account = None;
    accounts.retain_mut(|account| {
        account.is_current = account.user_id == current_user_id;
        if account.is_current {
            current_account = Some(account.clone());
            false
        } else {
            true
        }
    });

    accounts.insert(
        0,
        current_account.unwrap_or_else(|| AccountSummary {
            user_id: current_user_id.to_string(),
            is_current: true,
            sessions: 0,
            projects: 0,
            messages: 0,
            latest_at: None,
        }),
    );
    accounts
}

pub fn query_projects(conn: &Connection) -> CoreResult<Vec<ProjectSummary>> {
    let mut stmt = conn.prepare(
        r#"
        select p.project_id,
               coalesce(p.name, p.project_id) as name,
               p.user_id,
               coalesce(p.absolute_path, '') as absolute_path,
               count(cs.session_id) as active_sessions,
               coalesce(sum((select count(*) from chat_message m where m.session_id=cs.session_id)), 0) as messages,
               max(cs.updated_at) as latest_at
        from project p
        join chat_session cs on cs.project_id=p.project_id
        where cs.deleted_at=0
        group by p.project_id, p.name, p.user_id, p.absolute_path
        order by latest_at desc
        "#,
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectSummary {
                project_id: row.get(0)?,
                name: row.get(1)?,
                owner_user_id: row.get(2)?,
                path: row.get(3)?,
                active_sessions: row.get(4)?,
                messages: row.get(5)?,
                latest_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn query_conversations(conn: &Connection) -> CoreResult<Vec<ConversationSummary>> {
    let mut stmt = conn.prepare(
        r#"
        select cs.session_id,
               coalesce(nullif(cs.session_title, ''), cs.session_id) as title,
               cs.project_id,
               coalesce(p.name, p.project_id) as project_name,
               coalesce(p.absolute_path, '') as project_path,
               p.user_id,
               (select count(*) from chat_message m where m.session_id=cs.session_id and m.deleted_at=0) as messages,
               cs.updated_at,
               cs.deleted_at,
               cs.work_mode
        from chat_session cs
        join project p on p.project_id=cs.project_id
        where cs.deleted_at=0
        order by cs.updated_at desc
        "#,
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ConversationSummary {
                session_id: row.get(0)?,
                title: row.get(1)?,
                project_id: row.get(2)?,
                project_name: row.get(3)?,
                project_path: row.get(4)?,
                owner_user_id: row.get(5)?,
                messages: row.get(6)?,
                updated_at: row.get(7)?,
                deleted_at: row.get(8)?,
                work_mode: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn query_excluded_deleted_count(conn: &Connection) -> CoreResult<i64> {
    let count = conn
        .query_row(
            r#"
            with ids as (
              select session_id from chat_turn
              union select session_id from chat_message
              union select session_id from history_v2
            )
            select count(*)
            from ids
            left join chat_session cs on cs.session_id=ids.session_id
            where cs.session_id is null or cs.deleted_at<>0
            "#,
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    Ok(count)
}

pub fn fetch_project_user(conn: &Connection, project_id: &str) -> CoreResult<Option<String>> {
    Ok(conn
        .query_row(
            "select user_id from project where project_id=?",
            params![project_id],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn fetch_session_project(conn: &Connection, session_id: &str) -> CoreResult<Option<String>> {
    Ok(conn
        .query_row(
            "select project_id from chat_session where session_id=? and deleted_at=0",
            params![session_id],
            |row| row.get(0),
        )
        .optional()?)
}

pub fn table_exists(conn: &Connection, table: &str) -> CoreResult<bool> {
    let exists: i64 = conn.query_row(
        "select count(*) from sqlite_master where type='table' and name=?",
        params![table],
        |row| row.get(0),
    )?;
    Ok(exists > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_accounts_include_current_user_without_local_sessions() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            r#"
            create table project (
                project_id text primary key,
                user_id text not null
            );
            create table chat_session (
                session_id text primary key,
                project_id text not null,
                deleted_at integer not null default 0,
                updated_at integer
            );
            create table chat_message (
                session_id text not null
            );

            insert into project (project_id, user_id) values ('project-old', '9000000000000002');
            insert into chat_session (session_id, project_id, deleted_at, updated_at)
            values ('session-old', 'project-old', 0, 1000);
            insert into chat_message (session_id) values ('session-old'), ('session-old');
            "#,
        )
        .unwrap();

        let accounts = query_accounts(&conn, Some("9000000000000001")).unwrap();
        let targets = target_accounts(accounts.clone(), Some("9000000000000001"));

        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].user_id, "9000000000000002");
        assert!(targets.iter().any(|account| {
            account.user_id == "9000000000000001"
                && account.is_current
                && account.sessions == 0
                && account.projects == 0
                && account.messages == 0
        }));
    }
}
