import { invoke } from "@tauri-apps/api/core";
import { demoAnalysis, demoDiscovery, demoPlan, demoVerify } from "./demoData";
import type {
  AnalysisResult,
  AppDiscoveryResult,
  ApplyResult,
  DatabaseLocation,
  RollbackResult,
  TransferPlan,
  TransferSelection,
  VerifyResult,
} from "./types";

function isTauriRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

async function call<T>(command: string, args?: Record<string, unknown>, fallback?: T): Promise<T> {
  if (!isTauriRuntime()) {
    if (fallback !== undefined) return structuredClone(fallback);
    throw new Error("This action requires the Tauri desktop runtime.");
  }
  return invoke<T>(command, args);
}

export function scanInstallations() {
  return call<AppDiscoveryResult>("scan_installations", undefined, demoDiscovery);
}

export function manualDiscovery(input: {
  app_exe?: string | null;
  database_path?: string | null;
  backup_dir?: string | null;
}) {
  return call<AppDiscoveryResult>("manual_discovery", input, {
    ...demoDiscovery,
    app_exe: input.app_exe ?? demoDiscovery.app_exe,
    database_path: input.database_path ?? demoDiscovery.database_path,
    backup_dir: input.backup_dir ?? demoDiscovery.backup_dir,
  });
}

export function pickAppExe() {
  return call<string | null>("pick_app_exe", undefined, null);
}

export function pickDatabaseFile() {
  return call<string | null>("pick_database_file", undefined, null);
}

export function analyzeDatabase(location: DatabaseLocation) {
  return call<AnalysisResult>("analyze_database", { location }, demoAnalysis);
}

export function previewTransfer(selection: TransferSelection) {
  return call<TransferPlan>("preview_transfer", { selection }, demoPlan(selection));
}

export function applyTransfer(planId: string) {
  return call<ApplyResult>("apply_transfer", { planId });
}

export function verifyFrontend() {
  return call<VerifyResult>("verify_frontend", undefined, demoVerify);
}

export function rollback(backupId: string) {
  return call<RollbackResult>("rollback", { backupId });
}

export function openPath(path: string) {
  return call<void>("open_path", { path });
}

export function closeTrae() {
  return call<number>("close_trae", undefined, 0);
}
