import { UserRoundCheck } from "lucide-react";
import type { AccountSummary } from "../lib/types";
import { compactId, formatDate } from "../lib/utils";
import { Badge } from "./Badge";

interface AccountRailProps {
  accounts: AccountSummary[];
  selectedUserId: string;
  onSelect: (userId: string) => void;
}

export function AccountRail({ accounts, selectedUserId, onSelect }: AccountRailProps) {
  return (
    <aside className="flex min-h-0 flex-col rounded-lg border border-border bg-card shadow-panel">
      <div className="border-b border-border p-4">
        <h2 className="text-sm font-semibold text-slate-950">数据库用户</h2>
        <p className="mt-1 text-xs text-muted-foreground">选择来源用户，右侧只显示该用户的未删除会话。</p>
      </div>
      <div className="min-h-0 flex-1 space-y-2 overflow-auto p-3">
        {accounts.map((account) => {
          const selected = selectedUserId === account.user_id;
          return (
            <button
              key={account.user_id}
              className={`w-full rounded-lg border p-3 text-left transition ${
                selected ? "border-slate-950 bg-slate-950 text-white" : "border-border bg-white hover:bg-slate-50"
              }`}
              onClick={() => onSelect(account.user_id)}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-mono text-sm">{compactId(account.user_id)}</span>
                {account.is_current ? <Badge tone="green">当前</Badge> : null}
              </div>
              <div className={`mt-2 grid grid-cols-3 gap-1 text-xs ${selected ? "text-slate-200" : "text-muted-foreground"}`}>
                <span>{account.sessions} 会话</span>
                <span>{account.projects} 项目</span>
                <span>{account.messages} 消息</span>
              </div>
              <div className={`mt-2 flex items-center gap-1 text-xs ${selected ? "text-slate-300" : "text-muted-foreground"}`}>
                <UserRoundCheck className="h-3.5 w-3.5" />
                {formatDate(account.latest_at)}
              </div>
            </button>
          );
        })}
      </div>
    </aside>
  );
}
