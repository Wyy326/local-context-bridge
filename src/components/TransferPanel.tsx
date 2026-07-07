import * as Select from "@radix-ui/react-select";
import { ArrowDownUp, CheckCircle2, ChevronDown, ListChecks, ShieldAlert } from "lucide-react";
import type { ReactNode } from "react";
import type { AccountSummary, TransferMode, TransferPlan } from "../lib/types";
import { compactId, pluralCount } from "../lib/utils";
import { Badge } from "./Badge";
import { Button } from "./Button";

interface TransferPanelProps {
  accounts: AccountSummary[];
  targetUserId: string;
  mode: TransferMode;
  selectedProjects: number;
  selectedSessions: number;
  plan?: TransferPlan | null;
  canApply: boolean;
  onTargetChange: (value: string) => void;
  onModeChange: (mode: TransferMode) => void;
  onPreview: () => void;
  onApply: () => void;
}

export function TransferPanel({
  accounts,
  targetUserId,
  mode,
  selectedProjects,
  selectedSessions,
  plan,
  canApply,
  onTargetChange,
  onModeChange,
  onPreview,
  onApply,
}: TransferPanelProps) {
  const selectedCount = mode === "project" ? selectedProjects : selectedSessions;
  return (
    <aside className="flex min-h-0 flex-col rounded-lg border border-border bg-card shadow-panel">
      <div className="border-b border-border p-4">
        <h2 className="text-sm font-semibold text-slate-950">转移篮</h2>
        <p className="mt-1 text-xs text-muted-foreground">确认目标账号、粒度和预览结果后再写库。</p>
      </div>
      <div className="min-h-0 flex-1 space-y-4 overflow-auto p-4">
        <div>
          <label className="text-xs font-medium text-muted-foreground">目标用户</label>
          <Select.Root value={targetUserId} onValueChange={onTargetChange}>
            <Select.Trigger className="mt-2 flex h-10 w-full items-center justify-between rounded-md border border-input bg-white px-3 text-sm">
              <Select.Value placeholder="选择目标用户" />
              <Select.Icon>
                <ChevronDown className="h-4 w-4" />
              </Select.Icon>
            </Select.Trigger>
            <Select.Portal>
              <Select.Content className="z-50 min-w-[260px] rounded-md border border-border bg-white p-1 shadow-panel">
                <Select.Viewport>
                  {accounts.map((account) => (
                    <Select.Item
                      key={account.user_id}
                      value={account.user_id}
                      className="cursor-pointer rounded px-2 py-2 text-sm outline-none hover:bg-muted"
                    >
                      <Select.ItemText>
                        {compactId(account.user_id)}
                        {account.is_current ? " (当前)" : ""}
                        {account.sessions === 0 ? " · 无本地会话" : ""}
                      </Select.ItemText>
                    </Select.Item>
                  ))}
                </Select.Viewport>
              </Select.Content>
            </Select.Portal>
          </Select.Root>
        </div>

        <div>
          <label className="text-xs font-medium text-muted-foreground">转移粒度</label>
          <div className="mt-2 grid grid-cols-2 gap-2">
            <ModeButton active={mode === "project"} icon={<ArrowDownUp />} label="项目级" onClick={() => onModeChange("project")} />
            <ModeButton active={mode === "session"} icon={<ListChecks />} label="会话级" onClick={() => onModeChange("session")} />
          </div>
        </div>

        <div className="grid grid-cols-2 gap-2">
          <Metric label="项目" value={selectedProjects} />
          <Metric label="会话" value={selectedSessions} />
        </div>

        <div className="rounded-lg border border-amber-200 bg-amber-50 p-3 text-sm text-amber-900">
          <div className="flex items-center gap-2 font-medium">
            <ShieldAlert className="h-4 w-4" />
            写入前检查
          </div>
          <p className="mt-2 text-xs leading-5">
            执行转移会要求 TRAE 关闭，自动备份 live 数据库，并排除已删除和孤立记录。
          </p>
        </div>

        {plan ? (
          <div className="max-h-[220px] space-y-3 overflow-y-auto rounded-lg border border-border bg-slate-50 p-3 pr-2">
            <div className="flex items-center gap-2 text-sm font-medium text-slate-950">
              <CheckCircle2 className="h-4 w-4 text-emerald-600" />
              预览已生成
            </div>
            <div className="space-y-2 text-xs text-muted-foreground">
              <div>{pluralCount(plan.actions.length, "项动作")}</div>
              <div>{pluralCount(plan.changed_pages.length, "个页面将变化")}</div>
              <div className="break-all">备份: {plan.backup_path}</div>
            </div>
            <div className="space-y-2">
              {plan.actions.map((action, index) => (
                <div key={`${action.kind}-${index}`} className="rounded border border-border bg-white p-2 text-xs">
                  <div className="font-medium text-slate-900">{action.description}</div>
                  <div className="mt-1 text-muted-foreground">{action.session_ids.length} 个会话转入 {compactId(action.to_user_id)}</div>
                </div>
              ))}
            </div>
            {plan.warnings.map((warning) => (
              <Badge key={warning} tone="amber" className="whitespace-normal">
                {warning}
              </Badge>
            ))}
          </div>
        ) : null}
      </div>
      <div className="space-y-2 border-t border-border p-4">
        <Button className="w-full" disabled={!targetUserId || selectedCount === 0} onClick={onPreview}>
          预览迁移
        </Button>
        <Button className="w-full" variant="primary" disabled={!canApply} onClick={onApply}>
          执行转移
        </Button>
      </div>
    </aside>
  );
}

function ModeButton({ active, icon, label, onClick }: { active: boolean; icon: ReactNode; label: string; onClick: () => void }) {
  return (
    <button
      className={`flex h-10 items-center justify-center gap-2 rounded-md border text-sm font-medium transition ${
        active ? "border-slate-950 bg-slate-950 text-white" : "border-border bg-white text-slate-700 hover:bg-muted"
      }`}
      onClick={onClick}
    >
      <span className="[&>svg]:h-4 [&>svg]:w-4">{icon}</span>
      {label}
    </button>
  );
}

function Metric({ label, value }: { label: string; value: number }) {
  return (
    <div className="rounded-lg border border-border bg-white p-3">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className="mt-1 text-xl font-semibold text-slate-950">{value}</div>
    </div>
  );
}
