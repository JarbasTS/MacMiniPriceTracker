"use client";

import { useEffect, useState } from "react";
import { Plus } from "lucide-react";

import StoreFormModal from "@/components/StoreFormModal";
import { getStores, updateStore } from "@/lib/api";
import type { Store } from "@/lib/types";
import { cn } from "@/lib/utils";

export default function StoresPage() {
  const [stores, setStores] = useState<Store[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [modalOpen, setModalOpen] = useState(false);

  useEffect(() => {
    let alive = true;
    getStores()
      .then((data) => alive && setStores(data))
      .catch((e: Error) => alive && setError(e.message));
    return () => {
      alive = false;
    };
  }, []);

  async function toggle(store: Store) {
    setError(null);
    try {
      const updated = await updateStore(store.id, { is_active: !store.is_active });
      setStores((list) => list?.map((s) => (s.id === updated.id ? updated : s)) ?? null);
    } catch (e) {
      setError((e as Error).message);
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Lojas</h1>
        <button
          type="button"
          onClick={() => setModalOpen(true)}
          className="inline-flex items-center gap-1.5 rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white hover:bg-zinc-800"
        >
          <Plus className="size-4" aria-hidden /> Adicionar Nova Loja
        </button>
      </div>

      {error && (
        <p role="alert" className="rounded-md bg-red-50 px-3 py-2 text-sm text-red-700">
          {error}
        </p>
      )}

      {stores === null ? (
        !error && <p className="text-sm text-zinc-500">Carregando lojas…</p>
      ) : (
        <div className="overflow-x-auto rounded-lg border border-zinc-200 bg-white">
          <table className="w-full text-left text-sm">
            <thead className="border-b border-zinc-200 text-zinc-500">
              <tr>
                <th className="px-4 py-2 font-medium">Loja</th>
                <th className="px-4 py-2 font-medium">Tipo</th>
                <th className="px-4 py-2 font-medium">Intervalo</th>
                <th className="px-4 py-2 font-medium">Status</th>
              </tr>
            </thead>
            <tbody>
              {stores.map((s) => (
                <tr key={s.id} className="border-b border-zinc-100 last:border-0">
                  <td className="px-4 py-2">
                    <p className="font-medium">{s.name}</p>
                    <p className="text-xs text-zinc-500">{s.slug}</p>
                  </td>
                  <td className="px-4 py-2">
                    <code className="rounded bg-zinc-100 px-1 text-xs">{s.scraper_type}</code>
                  </td>
                  <td className="px-4 py-2">{s.scrape_interval_minutes} min</td>
                  <td className="px-4 py-2">
                    <button
                      type="button"
                      role="switch"
                      aria-checked={s.is_active}
                      aria-label={`${s.is_active ? "Desativar" : "Ativar"} ${s.name}`}
                      onClick={() => toggle(s)}
                      className={cn(
                        "inline-flex items-center gap-2 rounded-full px-2.5 py-1 text-xs font-medium",
                        s.is_active ? "bg-green-100 text-green-800" : "bg-zinc-100 text-zinc-600",
                      )}
                    >
                      <span className={cn("size-2 rounded-full", s.is_active ? "bg-green-600" : "bg-zinc-400")} />
                      {s.is_active ? "Ativa" : "Inativa"}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {modalOpen && (
        <StoreFormModal
          onClose={() => setModalOpen(false)}
          onCreated={(created) => {
            setStores((list) => [...(list ?? []), created]);
            setModalOpen(false);
          }}
        />
      )}
    </div>
  );
}
