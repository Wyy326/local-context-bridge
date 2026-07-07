import { useEffect, useState } from "react";

interface ToastItem {
  id: number;
  title: string;
  message: string;
}

export function toast(title: string, message: string) {
  window.dispatchEvent(new CustomEvent("lcb-toast", { detail: { title, message } }));
}

export function ToastHost() {
  const [items, setItems] = useState<ToastItem[]>([]);

  useEffect(() => {
    const onToast = (event: Event) => {
      const detail = (event as CustomEvent<{ title: string; message: string }>).detail;
      const id = Date.now();
      setItems((prev) => [...prev.slice(-2), { id, title: detail.title, message: detail.message }]);
      window.setTimeout(() => {
        setItems((prev) => prev.filter((item) => item.id !== id));
      }, 4200);
    };
    window.addEventListener("lcb-toast", onToast);
    return () => window.removeEventListener("lcb-toast", onToast);
  }, []);

  return (
    <div className="fixed bottom-20 right-6 z-50 flex w-[380px] flex-col gap-2">
      {items.map((item) => (
        <div key={item.id} className="rounded-lg border border-border bg-white p-4 shadow-panel">
          <div className="text-sm font-semibold text-slate-950">{item.title}</div>
          <div className="mt-1 text-sm text-muted-foreground">{item.message}</div>
        </div>
      ))}
    </div>
  );
}
