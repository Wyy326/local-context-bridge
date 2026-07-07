import { FolderOpen, RefreshCw, RotateCcw, ShieldX, Terminal, Undo2 } from "lucide-react";
import { Button } from "./Button";

interface BottomBarProps {
  busy: boolean;
  canPreview: boolean;
  canApply: boolean;
  onScan: () => void;
  onCloseTrae: () => void;
  onPreview: () => void;
  onApply: () => void;
  onVerify: () => void;
  onOpenBackup: () => void;
  onRollback: () => void;
  hasBackup: boolean;
}

export function BottomBar({
  busy,
  canPreview,
  canApply,
  hasBackup,
  onApply,
  onCloseTrae,
  onOpenBackup,
  onPreview,
  onRollback,
  onScan,
  onVerify,
}: BottomBarProps) {
  return (
    <footer className="border-t border-border bg-white">
      <div className="mx-auto flex h-[64px] max-w-[1500px] items-center justify-between px-6">
        <div className="text-sm text-muted-foreground">{busy ? "正在处理，请不要手动启动或关闭 TRAE。" : "所有写入动作都会先生成备份和预览报告。"}</div>
        <div className="flex items-center gap-2">
          <Button onClick={onScan}>
            <RefreshCw className="h-4 w-4" />
            重新扫描
          </Button>
          <Button onClick={onCloseTrae}>
            <ShieldX className="h-4 w-4" />
            关闭 TRAE
          </Button>
          <Button disabled={!canPreview} onClick={onPreview}>
            <Terminal className="h-4 w-4" />
            预览迁移
          </Button>
          <Button variant="primary" disabled={!canApply} onClick={onApply}>
            <FolderOpen className="h-4 w-4" />
            执行转移
          </Button>
          <Button onClick={onVerify}>
            <RotateCcw className="h-4 w-4" />
            验证前端
          </Button>
          <Button disabled={!hasBackup} onClick={onOpenBackup}>
            <FolderOpen className="h-4 w-4" />
            打开备份
          </Button>
          <Button disabled={!hasBackup} onClick={onRollback}>
            <Undo2 className="h-4 w-4" />
            回滚
          </Button>
        </div>
      </div>
    </footer>
  );
}
