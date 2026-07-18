mod command;
mod crypto;
mod database;
mod error;
mod paths;
mod process;
mod transfer;
mod types;

pub use crypto::KEY_HEX;
pub use database::analyze_database;
pub use error::{CoreError, CoreResult};
pub use paths::{manual_discovery, scan_installations};
pub use process::{close_trae_processes, is_trae_running, verify_frontend_logs};
pub use transfer::{apply_transfer, preview_transfer, rollback_backup};
pub use types::*;
