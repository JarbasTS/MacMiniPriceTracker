"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import { Search } from "lucide-react";

import ListingCard from "@/components/ListingCard";
import TriggerScrapeButton from "@/components/TriggerScrapeButton";
import { getListings, getStores, getSummary } from "@/lib/api";
import type { ListingView, StatsSummary, Store } from "@/lib/types";
import { CHIPS, cn, formatBRL, formatDateTime } from "@/lib/utils";

export default function Dashboard() {
  const [chip, setChip] = useState("");
  const [storeId, setStoreId] = useState("");
  const [query, setQuery] = useState("");
  // Incrementado após uma varredura para refazer as chamadas.
  const [reloadKey, setReloadKey] = useState(0);

  const [listings, setListings] = useState<ListingView[] | null>(null);
  const [summary, setSummary] = useState<StatsSummary | null>(null);
  const [stores, setStores] = useState<Store[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    getListings({ chip_family: chip, store_id: storeId })
      .then((data) => {
        if (!alive) return;
        setListings(data);
        setError(null);
      })
      .catch((e: Error) => alive && setError(e.message));
    return () => {
      alive = false;
    };
  }, [chip, storeId, reloadKey]);

  useEffect(() => {
    let alive = true;
    Promise.all([getSummary(), getStores()])
      .then(([s, st]) => {
        if (!alive) return;
        setSummary(s);
        setStores(st);
      })
      .catch((e: Error) => alive && setError(e.message));
    return () => {
      alive = false;
    };
  }, [reloadKey]);

  // CHIPS são os padrões; chips de alvos criados depois entram pelo resumo.
  const chips = [...new Set([...CHIPS, ...(summary?.by_chip.map((b) => b.chip_family) ?? [])])];

  const q = query.trim().toLowerCase();
  const visible = listings?.filter((l) => l.title.toLowerCase().includes(q)) ?? [];

  return (
    <div className="flex flex-col gap-6">
      <section className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4" aria-label="Menor preço por chip">
        {chips.map((c) => {
          const best = summary?.by_chip.find((b) => b.chip_family === c);
          return (
            <div key={c} className="rounded-lg border border-zinc-200 bg-white p-4">
              <p className="text-sm text-zinc-500">{/^M\d/.test(c) ? `Mac mini ${c}` : c}</p>
              {best ? (
                <Link href={`/listings/${best.listing_id}`} className="block hover:underline">
                  <p className="text-2xl font-semibold">{formatBRL(best.min_price)}</p>
                  <p className="text-sm text-zinc-600">{best.store_name}</p>
                </Link>
              ) : (
                <p className="text-2xl font-semibold text-zinc-400">—</p>
              )}
            </div>
          );
        })}
      </section>

      <section className="flex flex-wrap items-center gap-3" aria-label="Filtros">
        <div className="flex rounded-md border border-zinc-300 bg-white p-0.5" role="group" aria-label="Chip">
          {["", ...chips].map((c) => (
            <button
              key={c || "todos"}
              type="button"
              onClick={() => setChip(c)}
              aria-pressed={chip === c}
              className={cn("rounded px-3 py-1.5 text-sm", chip === c ? "bg-zinc-900 text-white" : "hover:bg-zinc-100")}
            >
              {c || "Todos"}
            </button>
          ))}
        </div>
        <select
          value={storeId}
          onChange={(e) => setStoreId(e.target.value)}
          aria-label="Loja"
          className="rounded-md border border-zinc-300 bg-white px-2.5 py-2 text-sm"
        >
          <option value="">Todas as lojas</option>
          {stores.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
        </select>
        <label className="relative flex-1 sm:max-w-xs">
          <span className="sr-only">Buscar por especificação</span>
          <Search className="absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-zinc-400" aria-hidden />
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="16GB, 512GB…"
            className="w-full rounded-md border border-zinc-300 bg-white py-2 pr-2.5 pl-8 text-sm"
          />
        </label>
        <div className="ml-auto">
          <TriggerScrapeButton onDone={() => setReloadKey((k) => k + 1)} />
        </div>
      </section>

      {summary && (
        <p className="text-xs text-zinc-500">
          {summary.total_listings} ofertas ativas · última varredura {formatDateTime(summary.last_scrape_at)}
        </p>
      )}

      {error ? (
        <p role="alert" className="rounded-md bg-red-50 px-3 py-2 text-sm text-red-700">
          Não foi possível carregar as ofertas: {error}
        </p>
      ) : listings === null ? (
        <p className="text-sm text-zinc-500">Carregando ofertas…</p>
      ) : visible.length === 0 ? (
        <p className="text-sm text-zinc-500">Nenhuma oferta encontrada com esses filtros.</p>
      ) : (
        <section className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3" aria-label="Ofertas">
          {visible.map((l) => (
            <ListingCard key={l.id} listing={l} />
          ))}
        </section>
      )}
    </div>
  );
}
