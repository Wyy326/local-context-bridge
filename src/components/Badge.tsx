import type { PropsWithChildren } from "react";
import { cn } from "../lib/utils";

interface BadgeProps {
  tone?: "neutral" | "green" | "amber" | "red" | "blue";
  className?: string;
}

const tones = {
  neutral: "border-slate-200 bg-slate-50 text-slate-600",
  green: "border-emerald-200 bg-emerald-50 text-emerald-700",
  amber: "border-amber-200 bg-amber-50 text-amber-700",
  red: "border-red-200 bg-red-50 text-red-700",
  blue: "border-blue-200 bg-blue-50 text-blue-700",
};

export function Badge({ children, className, tone = "neutral" }: PropsWithChildren<BadgeProps>) {
  return (
    <span className={cn("inline-flex items-center rounded-md border px-2 py-0.5 text-xs font-medium", tones[tone], className)}>
      {children}
    </span>
  );
}
