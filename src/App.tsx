import { useCallback, useEffect, useMemo, useState } from "react";
import { ToastHost, toast } from "./components/toast";
import { AccountRail } from "./components/AccountRail";
import { BottomBar } from "./components/BottomBar";
import { ConversationTable } from "./components/ConversationTable";
import { ShellHeader } from "./components/Shell";
import { TransferPanel } from "./components/TransferPanel";
import {
  analyzeDatabase,
  applyTransfer,
  closeTrae,
  manualDiscovery,
  openPath,
  pickAppExe,
  pickDatabaseFile,
  previewTransfer,
  rollback,
  scanInstallations,
  verifyFrontend,
} from "./lib/api";
import type { AnalysisResult, AppDiscoveryResult, TransferMode, TransferPlan } from "./lib/types";

export default function App() {
  const [busy, setBusy] = useState(false);
  const [discovery, setDiscovery] = useState<AppDiscoveryResult | null>(null);
  const [analysis, setAnalysis] = useState<AnalysisResult | null>(null);
  const [sourceUserId, setSourceUserId] = useState("");
  const [targetUserId, setTargetUserId] = useState("");
  const [mode, setMode] = useState<TransferMode>("session");
  const [search, setSearch] = useState("");
  const [selectedSessionIds, setSelectedSessionIds] = useState<Set<string>>(() => new Set());
  const [selectedProjectIds, setSelectedProjectIds] = useState<Set<string>>(() => new Set());
  const [plan, setPlan] = useState<TransferPlan | null>(null);
  const [lastBackupPath, setLastBackupPath] = useState("");

  const applyDiscovery = useCallback(async (found: AppDiscoveryResult) => {
    setDiscovery(found);
    if (!found.database_path) {
      setAnalysis(null);
      setSourceUserId("");
      setTargetUserId("");
      setSelectedProjectIds(new Set());
      setSelectedSessionIds(new Set());
      setPlan(null);
      toast("没有找到数据库", "右上角数据库卡片已切换为“浏览选择”，点击即可选择 database.db。");
      return;
    }
    const result = await analyzeDatabase({
      database_path: found.database_path,
      app_exe: found.app_exe,
      log_dir: found.log_dir,
      backup_dir: found.backup_dir,
    });
    setAnalysis(result);
    const target =
      result.default_target_user_id ??
      result.target_accounts.find((item) => item.is_current)?.user_id ??
      result.target_accounts[0]?.user_id ??
      "";
    setTargetUserId(target);
    const source = result.accounts.find((item) => item.user_id !== target)?.user_id ?? result.accounts[0]?.user_id ?? "";
    setSourceUserId(source);
    setSelectedProjectIds(new Set());
    setSelectedSessionIds(new Set());
    setPlan(null);
  }, []);

  const load = useCallback(async () => {
    setBusy(true);
    try {
      const found = await scanInstallations();
      await applyDiscovery(found);
    } catch (error) {
      toast("扫描失败", error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [applyDiscovery]);

  useEffect(() => {
    void load();
  }, [load]);

  const visibleConversations = useMemo(() => {
    const rows = analysis?.conversations ?? [];
    return rows.filter((item) => item.owner_user_id === sourceUserId && item.deleted_at === 0);
  }, [analysis?.conversations, sourceUserId]);

  const selectedProjectsCount = selectedProjectIds.size;
  const selectedSessionsCount = selectedSessionIds.size;
  const canPreview = !!analysis && !!targetUserId && (mode === "project" ? selectedProjectIds.size > 0 : selectedSessionIds.size > 0);
  const canApply = !!plan && canPreview && !busy;

  const toggleSession = useCallback((sessionId: string) => {
    setPlan(null);
    setSelectedSessionIds((prev) => {
      const next = new Set(prev);
      if (next.has(sessionId)) next.delete(sessionId);
      else next.add(sessionId);
      return next;
    });
  }, []);

  const toggleProject = useCallback((projectId: string) => {
    setPlan(null);
    setSelectedProjectIds((prev) => {
      const next = new Set(prev);
      if (next.has(projectId)) next.delete(projectId);
      else next.add(projectId);
      return next;
    });
  }, []);

  const clearSelection = useCallback(() => {
    setPlan(null);
    setSelectedProjectIds(new Set());
    setSelectedSessionIds(new Set());
  }, []);

  const selectVisible = useCallback(() => {
    setPlan(null);
    if (mode === "project") {
      setSelectedProjectIds(new Set(visibleConversations.map((item) => item.project_id)));
    } else {
      setSelectedSessionIds(new Set(visibleConversations.map((item) => item.session_id)));
    }
  }, [mode, visibleConversations]);

  const buildSelection = useCallback(() => {
    if (!analysis) throw new Error("数据库尚未分析");
    return {
      database_path: analysis.location.database_path,
      target_user_id: targetUserId,
      mode,
      selected_project_ids: [...selectedProjectIds],
      selected_session_ids: [...selectedSessionIds],
      backup_dir: analysis.location.backup_dir,
    };
  }, [analysis, mode, selectedProjectIds, selectedSessionIds, targetUserId]);

  const handlePreview = useCallback(async () => {
    setBusy(true);
    try {
      const result = await previewTransfer(buildSelection());
      setPlan(result);
      toast("预览完成", `将执行 ${result.actions.length} 项动作，影响 ${result.changed_pages.length} 个数据库页面。`);
    } catch (error) {
      toast("预览失败", error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [buildSelection]);

  const handleApply = useCallback(async () => {
    if (!plan) return;
    setBusy(true);
    try {
      const result = await applyTransfer(plan.plan_id);
      setLastBackupPath(result.backup_path);
      toast("转移完成", `完整性: ${result.integrity}，HMAC: ${result.hmac_status}`);
      setPlan(null);
      await load();
    } catch (error) {
      toast("转移失败", error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [load, plan]);

  const handleCloseTrae = useCallback(async () => {
    setBusy(true);
    try {
      const count = await closeTrae();
      toast("已请求关闭 TRAE", `关闭进程数: ${count}`);
    } catch (error) {
      toast("关闭失败", error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, []);

  const handlePickApp = useCallback(async () => {
    setBusy(true);
    try {
      const appExe = await pickAppExe();
      if (!appExe) return;
      const found = await manualDiscovery({
        app_exe: appExe,
        database_path: discovery?.database_path,
        backup_dir: discovery?.backup_dir,
      });
      await applyDiscovery(found);
      toast("APP 路径已更新", appExe);
    } catch (error) {
      toast("选择 APP 失败", error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [applyDiscovery, discovery?.backup_dir, discovery?.database_path]);

  const handlePickDatabase = useCallback(async () => {
    setBusy(true);
    try {
      const databasePath = await pickDatabaseFile();
      if (!databasePath) return;
      const found = await manualDiscovery({
        app_exe: discovery?.app_exe,
        database_path: databasePath,
        backup_dir: discovery?.backup_dir,
      });
      await applyDiscovery(found);
      toast("数据库路径已更新", databasePath);
    } catch (error) {
      toast("选择数据库失败", error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [applyDiscovery, discovery?.app_exe, discovery?.backup_dir]);

  const handleVerify = useCallback(async () => {
    setBusy(true);
    try {
      const result = await verifyFrontend();
      toast(
        result.malformed_count === 0 ? "验证通过" : "验证发现异常",
        result.malformed_count === 0 ? `读取到 ${result.success_lines.length} 条前端成功日志。` : `malformed 次数: ${result.malformed_count}`,
      );
    } catch (error) {
      toast("验证失败", error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, []);

  const handleOpenBackup = useCallback(async () => {
    if (!lastBackupPath && !plan?.backup_path) return;
    try {
      await openPath(lastBackupPath || plan?.backup_path || "");
    } catch (error) {
      toast("打开备份失败", error instanceof Error ? error.message : String(error));
    }
  }, [lastBackupPath, plan?.backup_path]);

  const handleRollback = useCallback(async () => {
    const backupPath = lastBackupPath || plan?.backup_path;
    if (!backupPath) return;
    setBusy(true);
    try {
      const result = await rollback(backupPath);
      toast(result.restored ? "回滚完成" : "没有可回滚文件", result.warnings[0] ?? backupPath);
      await load();
    } catch (error) {
      toast("回滚失败", error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  }, [lastBackupPath, load, plan?.backup_path]);

  return (
    <div className="flex h-dvh flex-col bg-background text-foreground">
      <ShellHeader discovery={discovery} busy={busy} onPickApp={handlePickApp} onPickDatabase={handlePickDatabase} />
      <main className="mx-auto grid min-h-0 w-full max-w-[1500px] flex-1 grid-cols-[280px_minmax(0,1fr)_330px] gap-4 px-6 py-4">
        <AccountRail
          accounts={analysis?.accounts ?? []}
          selectedUserId={sourceUserId}
          onSelect={(userId) => {
            setSourceUserId(userId);
            clearSelection();
          }}
        />
        <ConversationTable
          rows={visibleConversations}
          selectedSessionIds={selectedSessionIds}
          selectedProjectIds={selectedProjectIds}
          mode={mode}
          search={search}
          onSearch={setSearch}
          onToggleSession={toggleSession}
          onToggleProject={toggleProject}
          onSelectVisible={selectVisible}
          onClear={clearSelection}
        />
        <TransferPanel
          accounts={analysis?.target_accounts ?? []}
          targetUserId={targetUserId}
          mode={mode}
          selectedProjects={selectedProjectsCount}
          selectedSessions={selectedSessionsCount}
          plan={plan}
          canApply={canApply}
          onTargetChange={(value) => {
            setTargetUserId(value);
            setPlan(null);
          }}
          onModeChange={(value) => {
            setMode(value);
            setPlan(null);
          }}
          onPreview={handlePreview}
          onApply={handleApply}
        />
      </main>
      <BottomBar
        busy={busy}
        canPreview={canPreview}
        canApply={canApply}
        hasBackup={Boolean(lastBackupPath || plan?.backup_path)}
        onScan={load}
        onCloseTrae={handleCloseTrae}
        onPreview={handlePreview}
        onApply={handleApply}
        onVerify={handleVerify}
        onOpenBackup={handleOpenBackup}
        onRollback={handleRollback}
      />
      <ToastHost />
    </div>
  );
}
