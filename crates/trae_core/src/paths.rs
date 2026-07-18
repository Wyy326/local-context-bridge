use std::{
    collections::HashSet,
    env, fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use regex::Regex;
use walkdir::WalkDir;

use crate::{
    command::hidden_command,
    error::CoreResult,
    types::{AppDiscoveryResult, CurrentUserCandidate},
};

pub fn scan_installations() -> CoreResult<AppDiscoveryResult> {
    let app_data = env_path("APPDATA").map(|path| path.join("TRAE SOLO CN"));
    let database_path = app_data
        .as_ref()
        .map(|path| {
            path.join("ModularData")
                .join("ai-agent")
                .join("database.db")
        })
        .filter(|path| path.exists());
    let log_dir = app_data
        .as_ref()
        .map(|path| path.join("logs"))
        .filter(|path| path.exists());
    let workspace_dir = workspace_dir()?;
    let backup_dir = backup_dir()?;
    fs::create_dir_all(&workspace_dir)?;
    fs::create_dir_all(&backup_dir)?;

    let app_exe = discover_app_exe()?;
    let current_user_candidates = detect_current_user_candidates(log_dir.as_deref())?;
    let mut warnings = Vec::new();
    if app_exe.is_none() {
        warnings.push("未发现 TRAE SOLO CN 程序路径；仍可基于已发现数据库分析。".to_string());
    }
    if database_path.is_none() {
        warnings.push("未发现 TRAE SOLO CN database.db；需要用户手动指定路径。".to_string());
    }

    Ok(AppDiscoveryResult {
        provider: "TRAE SOLO CN".to_string(),
        app_exe: app_exe.map(path_string),
        app_data_dir: app_data.map(path_string),
        database_path: database_path.map(path_string),
        log_dir: log_dir.map(path_string),
        workspace_dir: path_string(workspace_dir),
        backup_dir: path_string(backup_dir),
        current_user_candidates,
        warnings,
    })
}

pub fn manual_discovery(
    app_exe: Option<String>,
    database_path: Option<String>,
    backup_dir_override: Option<String>,
) -> CoreResult<AppDiscoveryResult> {
    let app_exe_path = app_exe
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let database_path = database_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let app_data = database_path
        .as_ref()
        .and_then(|path| infer_app_data_dir_from_database_path(path));
    let log_dir = app_data
        .as_ref()
        .map(|path| path.join("logs"))
        .filter(|path| path.exists());
    let workspace_dir = workspace_dir()?;
    let backup_dir = backup_dir_override
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or(backup_dir()?);
    fs::create_dir_all(&workspace_dir)?;
    fs::create_dir_all(&backup_dir)?;

    let mut warnings = Vec::new();
    if app_exe_path.as_ref().is_some_and(|path| !path.exists()) {
        warnings.push("手动选择的 TRAE SOLO CN 程序路径不存在。".to_string());
    }
    if database_path.as_ref().is_some_and(|path| !path.exists()) {
        warnings.push("手动选择的 database.db 路径不存在。".to_string());
    }
    if database_path.is_some() && app_data.is_none() {
        warnings.push("已使用手动 database.db；未能按默认结构推导日志目录。".to_string());
    }

    let current_user_candidates = detect_current_user_candidates(log_dir.as_deref())?;

    Ok(AppDiscoveryResult {
        provider: "TRAE SOLO CN".to_string(),
        app_exe: app_exe_path.filter(|path| path.exists()).map(path_string),
        app_data_dir: app_data.map(path_string),
        database_path: database_path.filter(|path| path.exists()).map(path_string),
        log_dir: log_dir.map(path_string),
        workspace_dir: path_string(workspace_dir),
        backup_dir: path_string(backup_dir),
        current_user_candidates,
        warnings,
    })
}

pub fn infer_app_data_dir_from_database_path(database_path: &Path) -> Option<PathBuf> {
    if database_path.file_name()?.to_string_lossy() != "database.db" {
        return None;
    }
    let ai_agent = database_path.parent()?;
    if ai_agent.file_name()?.to_string_lossy() != "ai-agent" {
        return None;
    }
    let modular_data = ai_agent.parent()?;
    if modular_data.file_name()?.to_string_lossy() != "ModularData" {
        return None;
    }
    modular_data.parent().map(Path::to_path_buf)
}

pub fn workspace_dir() -> CoreResult<PathBuf> {
    if let Some(local) = env_path("LOCALAPPDATA") {
        return Ok(local.join("Local Context Bridge").join("workspace"));
    }
    Ok(env::temp_dir()
        .join("Local Context Bridge")
        .join("workspace"))
}

pub fn backup_dir() -> CoreResult<PathBuf> {
    if let Some(profile) = env_path("USERPROFILE") {
        return Ok(profile
            .join("Documents")
            .join("Local Context Bridge")
            .join("Backups"));
    }
    Ok(env::current_dir()?.join("backups"))
}

pub fn detect_current_user_candidates(
    log_dir: Option<&Path>,
) -> CoreResult<Vec<CurrentUserCandidate>> {
    let Some(log_dir) = log_dir else {
        return Ok(Vec::new());
    };
    if !log_dir.exists() {
        return Ok(Vec::new());
    }
    let patterns = [
        Regex::new(r#"user_id["']?\s*:\s*String\("(\d{6,})"\)"#)?,
        Regex::new(r#"biz_user_id["']?\s*:\s*String\("(\d{6,})"\)"#)?,
        Regex::new(r#"cached_user_id=Some\("(\d{6,})"\)"#)?,
        Regex::new(r#"user_id=(\d{6,})"#)?,
        Regex::new(r#""user_id"\s*:\s*"(\d{6,})""#)?,
        Regex::new(r#""biz_user_id"\s*:\s*"(\d{6,})""#)?,
    ];
    let mut files = Vec::new();
    for entry in WalkDir::new(log_dir).max_depth(4) {
        let entry = entry?;
        if entry.file_type().is_file() && entry.path().extension().is_some_and(|ext| ext == "log") {
            let modified = entry
                .metadata()?
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs_f64())
                .unwrap_or(0.0);
            files.push((entry.path().to_path_buf(), modified));
        }
    }
    files.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    files.truncate(80);

    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for (path, mtime) in files {
        let text = fs::read_to_string(&path).unwrap_or_default();
        for (line_index, line) in text.lines().enumerate() {
            for pattern in &patterns {
                for capture in pattern.captures_iter(line) {
                    let user_id = capture.get(1).map(|m| m.as_str()).unwrap_or_default();
                    if user_id == "0" || !seen.insert(user_id.to_string()) {
                        continue;
                    }
                    candidates.push(CurrentUserCandidate {
                        user_id: user_id.to_string(),
                        source: path_string(&path),
                        line: line.chars().take(260).collect(),
                        mtime: mtime + line_index as f64 / 1_000_000.0,
                    });
                }
            }
        }
    }
    candidates.sort_by(|a, b| {
        b.mtime
            .partial_cmp(&a.mtime)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(candidates)
}

fn discover_app_exe() -> CoreResult<Option<PathBuf>> {
    let mut candidates = Vec::new();
    if let Some(path) = running_process_path()? {
        candidates.push(path);
    }
    candidates.extend(registry_install_paths()?);
    candidates.extend(common_install_paths());
    Ok(candidates.into_iter().find(|path| path.exists()))
}

fn running_process_path() -> CoreResult<Option<PathBuf>> {
    let output = hidden_command("powershell")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            r#"$p = Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -like '*TRAE SOLO CN*' } | Select-Object -First 1 -ExpandProperty ExecutablePath; if ($p) { $p }"#,
        ])
        .output();
    let Ok(output) = output else {
        return Ok(None);
    };
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(text)))
    }
}

#[cfg(windows)]
fn registry_install_paths() -> CoreResult<Vec<PathBuf>> {
    use winreg::{enums::*, RegKey};
    let hives = [
        RegKey::predef(HKEY_CURRENT_USER),
        RegKey::predef(HKEY_LOCAL_MACHINE),
    ];
    let roots = [
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
        r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];
    let mut out = Vec::new();
    for hive in hives {
        for root in roots {
            if let Ok(key) = hive.open_subkey(root) {
                for name in key.enum_keys().flatten() {
                    if let Ok(app) = key.open_subkey(name) {
                        let display: String = app.get_value("DisplayName").unwrap_or_default();
                        if !display.contains("TRAE SOLO CN") {
                            continue;
                        }
                        let install: String = app.get_value("InstallLocation").unwrap_or_default();
                        if !install.is_empty() {
                            out.push(PathBuf::from(install).join("TRAE SOLO CN.exe"));
                        }
                        let display_icon: String = app.get_value("DisplayIcon").unwrap_or_default();
                        if !display_icon.is_empty() {
                            out.push(PathBuf::from(display_icon.trim_matches('"')));
                        }
                    }
                }
            }
        }
    }
    Ok(out)
}

#[cfg(not(windows))]
fn registry_install_paths() -> CoreResult<Vec<PathBuf>> {
    Ok(Vec::new())
}

fn common_install_paths() -> Vec<PathBuf> {
    let mut out = vec![PathBuf::from(r"D:\App\TRAE SOLO CN\TRAE SOLO CN.exe")];
    for key in ["LOCALAPPDATA", "PROGRAMFILES", "PROGRAMFILES(X86)"] {
        if let Some(base) = env_path(key) {
            out.push(base.join("TRAE SOLO CN").join("TRAE SOLO CN.exe"));
            out.push(
                base.join("Programs")
                    .join("TRAE SOLO CN")
                    .join("TRAE SOLO CN.exe"),
            );
        }
    }
    out
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key).map(PathBuf::from)
}

pub fn path_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::infer_app_data_dir_from_database_path;
    use std::path::Path;

    #[test]
    fn infers_app_data_dir_from_custom_drive_database_path() {
        let database_path =
            Path::new(r"X:\PortableData\TRAE SOLO CN\ModularData\ai-agent\database.db");

        let app_data = infer_app_data_dir_from_database_path(database_path).unwrap();

        assert_eq!(app_data, Path::new(r"X:\PortableData\TRAE SOLO CN"));
    }
}
