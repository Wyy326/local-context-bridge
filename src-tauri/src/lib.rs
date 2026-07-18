use std::{collections::HashMap, sync::Mutex};

use tauri::{async_runtime, Manager, State};
use trae_core::{
    AnalysisResult, AppDiscoveryResult, ApplyResult, DatabaseLocation, RollbackResult,
    TransferPlan, TransferSelection, VerifyResult,
};

#[derive(Default)]
struct PlanStore {
    selections: Mutex<HashMap<String, TransferSelection>>,
}

#[tauri::command]
async fn scan_installations() -> Result<AppDiscoveryResult, String> {
    run_blocking(|| trae_core::scan_installations().map_err(to_string)).await
}

#[tauri::command]
async fn manual_discovery(
    app_exe: Option<String>,
    database_path: Option<String>,
    backup_dir: Option<String>,
) -> Result<AppDiscoveryResult, String> {
    run_blocking(move || {
        trae_core::manual_discovery(app_exe, database_path, backup_dir).map_err(to_string)
    })
    .await
}

#[tauri::command]
fn pick_app_exe() -> Result<Option<String>, String> {
    Ok(rfd::FileDialog::new()
        .set_title("选择 TRAE SOLO CN.exe")
        .add_filter("TRAE SOLO CN", &["exe"])
        .pick_file()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
fn pick_database_file() -> Result<Option<String>, String> {
    Ok(rfd::FileDialog::new()
        .set_title("选择 database.db")
        .add_filter("SQLite database", &["db"])
        .pick_file()
        .map(|path| path.to_string_lossy().to_string()))
}

#[tauri::command]
async fn analyze_database(location: DatabaseLocation) -> Result<AnalysisResult, String> {
    run_blocking(move || trae_core::analyze_database(location).map_err(to_string)).await
}

#[tauri::command]
async fn preview_transfer(
    selection: TransferSelection,
    store: State<'_, PlanStore>,
) -> Result<TransferPlan, String> {
    let plan = run_blocking({
        let selection = selection.clone();
        move || trae_core::preview_transfer(selection).map_err(to_string)
    })
    .await?;
    store
        .selections
        .lock()
        .map_err(|_| "plan store is poisoned".to_string())?
        .insert(plan.plan_id.clone(), selection);
    Ok(plan)
}

#[tauri::command]
async fn apply_transfer(
    plan_id: String,
    store: State<'_, PlanStore>,
) -> Result<ApplyResult, String> {
    let selection = store
        .selections
        .lock()
        .map_err(|_| "plan store is poisoned".to_string())?
        .get(&plan_id)
        .cloned()
        .ok_or_else(|| "plan_id not found; generate a preview first".to_string())?;
    run_blocking(move || trae_core::apply_transfer(&selection, &plan_id).map_err(to_string)).await
}

#[tauri::command]
async fn verify_frontend() -> Result<VerifyResult, String> {
    run_blocking(|| trae_core::verify_frontend_logs().map_err(to_string)).await
}

#[tauri::command]
async fn rollback(backup_id: String) -> Result<RollbackResult, String> {
    run_blocking(move || trae_core::rollback_backup(&backup_id).map_err(to_string)).await
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("path is empty".to_string());
    }
    #[cfg(windows)]
    {
        let mut command = hidden_command("explorer");
        command.arg(path).spawn().map_err(to_string)?;
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(to_string)?;
    }
    Ok(())
}

#[tauri::command]
async fn close_trae() -> Result<usize, String> {
    run_blocking(|| trae_core::close_trae_processes().map_err(to_string)).await
}

pub fn run() {
    tauri::Builder::default()
        .manage(PlanStore::default())
        .invoke_handler(tauri::generate_handler![
            scan_installations,
            manual_discovery,
            pick_app_exe,
            pick_database_file,
            analyze_database,
            preview_transfer,
            apply_transfer,
            verify_frontend,
            rollback,
            open_path,
            close_trae
        ])
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_title("Local Context Bridge");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Local Context Bridge");
}

fn to_string(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(windows)]
fn hidden_command(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut command = std::process::Command::new(program);
    command.creation_flags(CREATE_NO_WINDOW);
    command
}

async fn run_blocking<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    async_runtime::spawn_blocking(task)
        .await
        .map_err(to_string)?
}
