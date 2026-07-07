import * as Checkbox from "@radix-ui/react-checkbox";
import {
  type ColumnDef,
  flexRender,
  getCoreRowModel,
  getFilteredRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import { Check, Search } from "lucide-react";
import { useMemo } from "react";
import type { ConversationSummary, TransferMode } from "../lib/types";
import { compactId, formatDate } from "../lib/utils";
import { Badge } from "./Badge";
import { Button } from "./Button";

interface ConversationTableProps {
  rows: ConversationSummary[];
  selectedSessionIds: Set<string>;
  selectedProjectIds: Set<string>;
  mode: TransferMode;
  search: string;
  onSearch: (value: string) => void;
  onToggleSession: (sessionId: string) => void;
  onToggleProject: (projectId: string) => void;
  onSelectVisible: () => void;
  onClear: () => void;
}

export function ConversationTable({
  rows,
  selectedSessionIds,
  selectedProjectIds,
  mode,
  search,
  onSearch,
  onToggleSession,
  onToggleProject,
  onSelectVisible,
  onClear,
}: ConversationTableProps) {
  const columns = useMemo<ColumnDef<ConversationSummary>[]>(
    () => [
      {
        id: "select",
        header: "",
        cell: ({ row }) => {
          const item = row.original;
          const checked =
            mode === "project" ? selectedProjectIds.has(item.project_id) : selectedSessionIds.has(item.session_id);
          return (
            <Checkbox.Root
              className="flex h-4 w-4 items-center justify-center rounded border border-slate-300 bg-white data-[state=checked]:border-slate-950 data-[state=checked]:bg-slate-950"
              checked={checked}
              onCheckedChange={() =>
                mode === "project" ? onToggleProject(item.project_id) : onToggleSession(item.session_id)
              }
              aria-label="选择会话"
            >
              <Checkbox.Indicator>
                <Check className="h-3 w-3 text-white" />
              </Checkbox.Indicator>
            </Checkbox.Root>
          );
        },
      },
      {
        accessorKey: "title",
        header: "会话",
        cell: ({ row }) => (
          <div className="min-w-[240px]">
            <div className="font-medium text-slate-950">{row.original.title || row.original.session_id}</div>
            <div className="mt-1 font-mono text-xs text-muted-foreground">{compactId(row.original.session_id, 8, 6)}</div>
          </div>
        ),
      },
      {
        accessorKey: "project_name",
        header: "项目",
        cell: ({ row }) => (
          <div className="min-w-[220px]">
            <div className="flex items-center gap-2">
              <span className="text-sm text-slate-900">{row.original.project_name || "未命名项目"}</span>
              {selectedProjectIds.has(row.original.project_id) ? <Badge tone="blue">项目选中</Badge> : null}
            </div>
            <div className="mt-1 max-w-[360px] truncate text-xs text-muted-foreground">{row.original.project_path}</div>
          </div>
        ),
      },
      {
        accessorKey: "messages",
        header: "消息",
        cell: ({ row }) => <span className="tabular-nums">{row.original.messages}</span>,
      },
      {
        accessorKey: "work_mode",
        header: "模式",
        cell: ({ row }) => <Badge>{row.original.work_mode || "default"}</Badge>,
      },
      {
        accessorKey: "updated_at",
        header: "更新时间",
        cell: ({ row }) => <span className="text-muted-foreground">{formatDate(row.original.updated_at)}</span>,
      },
    ],
    [mode, onToggleProject, onToggleSession, selectedProjectIds, selectedSessionIds],
  );

  const table = useReactTable({
    data: rows,
    columns,
    state: {
      globalFilter: search,
    },
    onGlobalFilterChange: onSearch,
    getCoreRowModel: getCoreRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  return (
    <section className="flex min-h-0 flex-col rounded-lg border border-border bg-card shadow-panel">
      <div className="flex items-center justify-between gap-3 border-b border-border p-4">
        <div>
          <h2 className="text-sm font-semibold text-slate-950">未删除会话</h2>
          <p className="mt-1 text-xs text-muted-foreground">
            {mode === "project" ? "项目模式会移动整个项目的未删除会话。" : "会话模式会在必要时克隆项目，避免带走未选中会话。"}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <div className="relative">
            <Search className="pointer-events-none absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
            <input
              className="h-9 w-[260px] rounded-md border border-input bg-white pl-8 pr-3 text-sm outline-none ring-offset-background focus:ring-2 focus:ring-ring"
              value={search}
              onChange={(event) => onSearch(event.target.value)}
              placeholder="搜索标题、项目、路径"
            />
          </div>
          <Button onClick={onSelectVisible}>选中可见</Button>
          <Button variant="ghost" onClick={onClear}>
            清空
          </Button>
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        <table className="w-full border-collapse text-sm">
          <thead className="sticky top-0 z-10 bg-slate-50 text-left text-xs font-medium uppercase text-muted-foreground">
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id}>
                {headerGroup.headers.map((header) => (
                  <th key={header.id} className="border-b border-border px-3 py-3">
                    {header.isPlaceholder ? null : flexRender(header.column.columnDef.header, header.getContext())}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <tr key={row.id} className="border-b border-border/70 hover:bg-slate-50">
                {row.getVisibleCells().map((cell) => (
                  <td key={cell.id} className="px-3 py-3 align-top">
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
        {table.getRowModel().rows.length === 0 ? (
          <div className="flex h-48 items-center justify-center text-sm text-muted-foreground">没有匹配的未删除会话</div>
        ) : null}
      </div>
    </section>
  );
}
