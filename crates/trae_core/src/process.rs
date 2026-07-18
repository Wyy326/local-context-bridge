use std::{
    fs,
    path::{Path, PathBuf},
};

use walkdir::WalkDir;

use crate::{command::hidden_command, error::CoreResult, paths, types::VerifyResult};

pub fn is_trae_running() -> bool {
    let output = hidden_command("powershell")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            r#"$items = Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -like '*TRAE SOLO CN*' }; @($items).Count"#,
        ])
        .output();
    let Ok(output) = output else {
        return false;
    };
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()
        .map(|count| count > 0)
        .unwrap_or(false)
}

pub fn close_trae_processes() -> CoreResult<usize> {
    let output = hidden_command("powershell")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            r#"$items = Get-CimInstance Win32_Process | Where-Object { $_.ExecutablePath -like '*TRAE SOLO CN*' }; $count=0; foreach ($item in $items) { Stop-Process -Id $item.ProcessId -Force -ErrorAction SilentlyContinue; $count += 1 }; $count"#,
        ])
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()
        .unwrap_or(0))
}

pub fn verify_frontend_logs() -> CoreResult<VerifyResult> {
    let log_dir = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("TRAE SOLO CN").join("logs"));
    let Some(log_dir) = log_dir else {
        return Ok(VerifyResult {
            log_dir: None,
            malformed_count: 0,
            success_lines: Vec::new(),
            error_lines: Vec::new(),
            warnings: vec!["APPDATA is not set.".to_string()],
        });
    };
    if !log_dir.exists() {
        return Ok(VerifyResult {
            log_dir: Some(paths::path_string(log_dir)),
            malformed_count: 0,
            success_lines: Vec::new(),
            error_lines: Vec::new(),
            warnings: vec!["TRAE log directory not found.".to_string()],
        });
    }
    let latest = latest_log_dir(&log_dir);
    let Some(latest) = latest else {
        return Ok(VerifyResult {
            log_dir: Some(paths::path_string(log_dir)),
            malformed_count: 0,
            success_lines: Vec::new(),
            error_lines: Vec::new(),
            warnings: vec!["No TRAE log sessions found.".to_string()],
        });
    };
    let mut malformed_count = 0usize;
    let mut success_lines = Vec::new();
    let mut error_lines = Vec::new();
    for entry in WalkDir::new(&latest).max_depth(5) {
        let entry = entry?;
        if !entry.file_type().is_file() || !entry.path().extension().is_some_and(|ext| ext == "log")
        {
            continue;
        }
        let text = fs::read_to_string(entry.path()).unwrap_or_default();
        malformed_count += text.matches("database disk image is malformed").count();
        for line in text.lines() {
            if line.contains("[lite][list_chat_sessions] result")
                || line.contains("[lite][list_projects] result")
            {
                success_lines.push(format!(
                    "{}: {}",
                    entry
                        .path()
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("log"),
                    line.chars().take(260).collect::<String>()
                ));
            }
            if line.contains("database disk image is malformed") {
                error_lines.push(format!(
                    "{}: {}",
                    entry
                        .path()
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("log"),
                    line.chars().take(260).collect::<String>()
                ));
            }
        }
    }
    success_lines = tail(success_lines, 20);
    error_lines = tail(error_lines, 20);
    let warnings = if malformed_count > 0 {
        vec!["TRAE logs still contain database disk image is malformed.".to_string()]
    } else {
        Vec::new()
    };
    Ok(VerifyResult {
        log_dir: Some(paths::path_string(latest)),
        malformed_count,
        success_lines,
        error_lines,
        warnings,
    })
}

fn latest_log_dir(root: &Path) -> Option<PathBuf> {
    let mut dirs = fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|ty| ty.is_dir()).unwrap_or(false))
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((entry.path(), modified))
        })
        .collect::<Vec<_>>();
    dirs.sort_by(|a, b| b.1.cmp(&a.1));
    dirs.into_iter().map(|(path, _)| path).next()
}

fn tail<T>(items: Vec<T>, max: usize) -> Vec<T> {
    let len = items.len();
    if len <= max {
        items
    } else {
        items.into_iter().skip(len - max).collect()
    }
}
