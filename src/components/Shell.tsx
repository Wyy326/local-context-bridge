import { Activity, Database, FolderSearch, ShieldCheck } from "lucide-react";
import type { ReactNode } from "react";
import type { AppDiscoveryResult } from "../lib/types";
import { Badge } from "./Badge";

interface ShellProps {
  discovery?: AppDiscoveryResult | null;
  busy: boolean;
  onPickApp?: () => void;
  onPickDatabase?: () => void;
}

export function ShellHeader({ discovery, busy, onPickApp, onPickDatabase }: ShellProps) {
  const current = discovery?.current_user_candidates[0]?.user_id;
  const appStatus = discovery ? (discovery.app_exe ? "已发现" : "浏览选择") : "待扫描";
  const databaseStatus = discovery ? (discovery.database_path ? "已定位" : "浏览选择") : "待扫描";
  return (
    <header className="border-b border-border bg-white">
      <div className="mx-auto flex h-[88px] max-w-[1500px] items-center justify-between px-6">
        <div>
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-slate-950 text-white">
              <ShieldCheck className="h-5 w-5" />
            </div>
            <div>
              <h1 className="text-xl font-semibold tracking-normal text-slate-950">Local Context Bridge</h1>
              <p className="text-sm text-muted-foreground">本地对话上下文跨账号转移工具</p>
            </div>
          </div>
        </div>
        <div className="grid min-w-[620px] grid-cols-4 gap-2">
          <StatusItem
            icon={<FolderSearch />}
            label="APP"
            value={appStatus}
            onClick={onPickApp}
            disabled={busy}
            title="选择 TRAE SOLO CN.exe"
          />
          <StatusItem
            icon={<Database />}
            label="数据库"
            value={databaseStatus}
            onClick={onPickDatabase}
            disabled={busy}
            title="选择 database.db"
          />
          <StatusItem icon={<Activity />} label="当前用户" value={current ?? "未识别"} />
          <StatusItem icon={<ShieldCheck />} label="状态" value={busy ? "处理中" : "就绪"} />
        </div>
      </div>
      {discovery?.warnings?.length ? (
        <div className="border-t border-amber-200 bg-amber-50 px-6 py-2 text-sm text-amber-900">
          <div className="mx-auto flex max-w-[1500px] flex-wrap items-center gap-x-4 gap-y-1">
            {discovery.warnings.map((warning) => (
              <span key={warning}>{warning}</span>
            ))}
            <span className="font-medium">可点击右上角 APP 或数据库卡片手动浏览选择。</span>
          </div>
        </div>
      ) : null}
    </header>
  );
}

function StatusItem({
  icon,
  label,
  value,
  onClick,
  disabled = false,
  title,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  onClick?: () => void;
  disabled?: boolean;
  title?: string;
}) {
  const content = (
    <>
      <div className="flex items-center gap-2 text-xs text-muted-foreground">
        <span className="[&>svg]:h-3.5 [&>svg]:w-3.5">{icon}</span>
        {label}
      </div>
      <div className="mt-1">
        <Badge tone={value === "浏览选择" || value === "未识别" ? "amber" : "blue"}>{value}</Badge>
      </div>
    </>
  );
  if (onClick) {
    return (
      <button
        type="button"
        className="rounded-lg border border-border bg-slate-50 px-3 py-2 text-left transition hover:bg-white disabled:cursor-not-allowed disabled:opacity-60"
        onClick={onClick}
        disabled={disabled}
        title={title}
        aria-label={title ?? label}
      >
        {content}
      </button>
    );
  }
  return (
    <div className="rounded-lg border border-border bg-slate-50 px-3 py-2">
      {content}
    </div>
  );
}
