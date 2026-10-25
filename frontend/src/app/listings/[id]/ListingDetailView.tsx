"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useEffect, useState } from "react";
import { ArrowLeft, ExternalLink } from "lucide-react";

import ChipBadge from "@/components/ChipBadge";
import PriceHistoryChart from "@/components/PriceHistoryChart";
import { getListing } from "@/lib/api";
import type { ListingDetail } from "@/lib/types";
import { formatBRL, formatDateTime } from "@/lib/utils";

export default function ListingDetailView() {
  const { id } = useParams<{ id: string }>();
  const [listing, setListing] = useState<ListingDetail | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    getListing(id)
      .then((data) => alive && setListing(data))
      .catch((e: Error) => alive && setError(e.message));
    return () => {
      alive = false;
    };
  }, [id]);

  const back = (
    <Link href="/" className="inline-flex items-center gap-1 text-sm text-zinc-600 hover:text-zinc-900">
      <ArrowLeft className="size-4" aria-hidden /> Voltar ao dashboard
    </Link>
  );

  if (error)
    return (
      <div className="flex flex-col gap-4">
        {back}
        <p role="alert" className="rounded-md bg-red-50 px-3 py-2 text-sm text-red-700">
          {error}
        </p>
      </div>
    );
  if (!listing) return <p className="text-sm text-zinc-500">Carregando…</p>;

  return (
    <div className="flex flex-col gap-6">
      {back}
      <header className="flex flex-col gap-4 rounded-lg border border-zinc-200 bg-white p-5 sm:flex-row">
        {listing.image_url && (
          <img src={listing.image_url} alt="" className="h-32 w-32 shrink-0 self-center object-contain" />
        )}
        <div className="flex flex-1 flex-col gap-2">
          <div className="flex flex-wrap items-center gap-1.5">
            <span className="rounded-full bg-zinc-900 px-2 py-0.5 text-xs font-medium text-white">{listing.store_name}</span>
            <ChipBadge chip={listing.chip_family} />
          </div>
          <h1 className="text-lg font-semibold">{listing.title}</h1>
          {listing.seller_name && <p className="text-sm text-zinc-500">Vendido por {listing.seller_name}</p>}
          <dl className="grid grid-cols-3 gap-3 text-sm sm:max-w-md">
            <div>
              <dt className="text-zinc-500">Atual</dt>
              <dd className="text-xl font-semibold">{formatBRL(listing.current_price)}</dd>
            </div>
            <div>
              <dt className="text-zinc-500">Menor</dt>
              <dd className="font-medium text-green-700">{formatBRL(listing.lowest_price)}</dd>
            </div>
            <div>
              <dt className="text-zinc-500">Maior</dt>
              <dd className="font-medium">{formatBRL(listing.highest_price)}</dd>
            </div>
          </dl>
          <p className="text-xs text-zinc-500">
            Visto pela primeira vez {formatDateTime(listing.first_seen_at)} · última {formatDateTime(listing.last_seen_at)}
          </p>
          <a
            href={listing.url}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex w-fit items-center gap-1 rounded-md bg-zinc-900 px-3 py-1.5 text-sm text-white hover:bg-zinc-800"
          >
            <ExternalLink className="size-4" aria-hidden /> Ver na loja
          </a>
        </div>
      </header>
      <section className="rounded-lg border border-zinc-200 bg-white p-5">
        <h2 className="mb-3 font-semibold">Evolução do preço</h2>
        <PriceHistoryChart history={listing.history} />
      </section>
    </div>
  );
}
