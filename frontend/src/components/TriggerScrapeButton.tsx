"use client";

import { useEffect, useRef, useState } from "react";
import { LoaderCircle, RefreshCw } from "lucide-react";

import { ApiError, triggerScrape } from "@/lib/api";

/** Dispara a varredura (202) e recarrega os dados ~5 s depois; 409 = já há uma rodando. */
export default function TriggerScrapeButton({ onDone }: { onDone: () => void }) {
  const [running, setRunning] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => () => clearTimeout(timer.current), []);

  async function handleClick() {
    setRunning(true);
    setMessage(null);
    try {
      await triggerScrape();
      setMessage("Varredura iniciada");
    } catch (e) {
      setMessage(e instanceof ApiError && e.status === 409 ? "Varredura já em andamento" : (e as Error).message);
    }
    timer.current = setTimeout(() => {
      setRunning(false);
      onDone();
    }, 5000);
  }

  return (
    <div className="flex items-center gap-2">
      <button
        type="button"
        onClick={handleClick}
        disabled={running}
        className="inline-flex items-center gap-2 rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white hover:bg-zinc-800 disabled:opacity-60"
      >
        {running ? <LoaderCircle className="size-4 animate-spin" aria-hidden /> : <RefreshCw className="size-4" aria-hidden />}
        Atualizar Preços Agora
      </button>
      {message && (
        <span role="status" className="text-sm text-zinc-600">
          {message}
        </span>
      )}
    </div>
  );
}
