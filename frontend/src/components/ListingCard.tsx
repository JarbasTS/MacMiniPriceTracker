import Link from "next/link";
import { ExternalLink, ChartLine } from "lucide-react";

import ChipBadge from "@/components/ChipBadge";
import type { ListingView } from "@/lib/types";
import { formatBRL } from "@/lib/utils";

export default function ListingCard({ listing }: { listing: ListingView }) {
  const isLowest = Number(listing.current_price) <= Number(listing.lowest_price);
  return (
    <article className="flex flex-col overflow-hidden rounded-lg border border-zinc-200 bg-white">
      <div className="flex h-40 items-center justify-center bg-zinc-50 p-3">
        {listing.image_url ? (
          <img src={listing.image_url} alt="" className="max-h-full max-w-full object-contain" loading="lazy" />
        ) : (
          <span className="text-sm text-zinc-400">Sem imagem</span>
        )}
      </div>
      <div className="flex flex-1 flex-col gap-2 p-4">
        <div className="flex flex-wrap items-center gap-1.5">
          <span className="rounded-full bg-zinc-900 px-2 py-0.5 text-xs font-medium text-white">{listing.store_name}</span>
          <ChipBadge chip={listing.chip_family} />
          {isLowest && (
            <span className="rounded-full bg-green-600 px-2 py-0.5 text-xs font-medium text-white">Menor preço</span>
          )}
        </div>
        <h3 className="line-clamp-2 text-sm text-zinc-800" title={listing.title}>
          {listing.title}
        </h3>
        {listing.seller_name && <p className="text-xs text-zinc-500">Vendido por {listing.seller_name}</p>}
        <div className="mt-auto">
          <p className="text-xl font-semibold text-zinc-900">{formatBRL(listing.current_price)}</p>
          <p className="text-xs text-zinc-500">Menor histórico: {formatBRL(listing.lowest_price)}</p>
        </div>
        <div className="flex gap-2 pt-1 text-sm">
          <a
            href={listing.url}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 rounded-md border border-zinc-300 px-2.5 py-1 text-zinc-700 hover:bg-zinc-50"
          >
            <ExternalLink className="size-3.5" aria-hidden /> Loja
          </a>
          <Link
            href={`/listings/${listing.id}`}
            className="inline-flex items-center gap-1 rounded-md border border-zinc-300 px-2.5 py-1 text-zinc-700 hover:bg-zinc-50"
          >
            <ChartLine className="size-3.5" aria-hidden /> Histórico
          </Link>
        </div>
      </div>
    </article>
  );
}
