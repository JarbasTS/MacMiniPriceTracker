"use client";

import { CartesianGrid, Legend, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";

import type { PriceHistory } from "@/lib/types";
import { formatBRL, formatDateTime } from "@/lib/utils";

const shortDate = new Intl.DateTimeFormat("pt-BR", { day: "2-digit", month: "2-digit" });

export default function PriceHistoryChart({ history }: { history: PriceHistory[] }) {
  const data = history.map((h) => ({
    t: new Date(h.captured_at).getTime(),
    cash: Number(h.price_cash),
    installments: h.price_installments === null ? null : Number(h.price_installments),
  }));
  const hasInstallments = data.some((d) => d.installments !== null);

  if (data.length === 0) return <p className="text-sm text-zinc-500">Sem histórico ainda.</p>;

  return (
    <div className="h-80 w-full">
      <ResponsiveContainer>
        <LineChart data={data} margin={{ top: 8, right: 16, bottom: 8, left: 16 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#e4e4e7" />
          <XAxis
            dataKey="t"
            type="number"
            scale="time"
            domain={["dataMin", "dataMax"]}
            tickFormatter={(t: number) => shortDate.format(t)}
            fontSize={12}
          />
          <YAxis
            domain={["auto", "auto"]}
            tickFormatter={(v: number) => formatBRL(v).replace(",00", "")}
            width={96}
            fontSize={12}
          />
          <Tooltip formatter={(v) => formatBRL(Number(v))} labelFormatter={(t) => formatDateTime(Number(t))} />
          <Legend />
          <Line type="stepAfter" dataKey="cash" name="À vista" stroke="#059669" strokeWidth={2} dot />
          {hasInstallments && (
            <Line
              type="stepAfter"
              dataKey="installments"
              name="Parcelado"
              stroke="#7c3aed"
              strokeWidth={2}
              strokeDasharray="5 3"
              dot
              connectNulls
            />
          )}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
