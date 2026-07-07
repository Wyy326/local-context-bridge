import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatDate(value?: number | null) {
  if (!value) return "未记录";
  const ms = value < 10_000_000_000 ? value * 1000 : value;
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(ms));
}

export function compactId(value: string, left = 6, right = 4) {
  if (!value) return "";
  if (value.length <= left + right + 2) return value;
  return `${value.slice(0, left)}...${value.slice(-right)}`;
}

export function pluralCount(count: number, label: string) {
  return `${count.toLocaleString("zh-CN")} ${label}`;
}
