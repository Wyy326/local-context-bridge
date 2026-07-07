use std::{collections::HashMap, sync::Mutex};

use tauri::{Manager, State};
use trae_core::{
    AnalysisResult, AppDiscoveryResult, ApplyResult, DatabaseLocation, RollbackResult,
    TransferPlan, TransferSelection, VerifyResult,
};

#[derive(Default)]
struct PlanStore {
    selections: Mutex<HashMap<String, TransferSelection>>,
}

#[tauri::command]
fn scan_installations() -> Result<AppDiscoveryResult, String> {
    trae_core::scan_installations().map_err(to_string)
}

#[tauri::command]
fn manual_discovery(
    app_exe: Option<String>,
    database_path: Option<String>,
    backup_dir: Option<String>,
) -> Result<AppDiscoveryResult, String> {
    trae_core::manual_discovery(app_exe, database_path, backup_dir).map_err(to_string)
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
fn analyze_database(location: DatabaseLocation) -> Result<AnalysisResult, String> {
    trae_core::analyze_database(location).map_err(to_string)
}

#[tauri::command]
fn preview_transfer(
    selection: TransferSelection,
    store: State<'_, PlanStore>,
) -> Result<TransferPlan, String> {
    let plan = trae_core::preview_transfer(selection.clone()).map_err(to_string)?;
    store
        .selections
        .lock()
        .map_err(|_| "plan store is poisoned".to_string())?
        .insert(plan.plan_id.clone(), selection);
    Ok(plan)
}

#[tauri::command]
fn apply_transfer(plan_id: String, store: State<'_, PlanStore>) -> Result<ApplyResult, String> {
    let selection = store
        .selections
        .lock()
        .map_err(|_| "plan store is poisoned".to_string())?
        .get(&plan_id)
        .cloned()
        .ok_or_else(|| "plan_id not found; generate a preview first".to_string())?;
    trae_core::apply_transfer(&selection, &plan_id).map_err(to_string)
}

#[tauri::command]
fn verify_frontend() -> Result<VerifyResult, String> {
    trae_core::verify_frontend_logs().map_err(to_string)
}

#[tauri::command]
fn rollback(backup_id: String) -> Result<RollbackResult, String> {
    trae_core::rollback_backup(&backup_id).map_err(to_string)
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("path is empty".to_string());
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer")
            .arg(path)
            .spawn()
            .map_err(to_string)?;
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
fn close_trae() -> Result<usize, String> {
    trae_core::close_trae_processes().map_err(to_string)
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
