import { cn } from "@/lib/utils";

const COLORS: Record<string, string> = {
  M5: "bg-sky-100 text-sky-800",
  "M5 Pro": "bg-violet-100 text-violet-800",
  M6: "bg-emerald-100 text-emerald-800",
};

export default function ChipBadge({ chip }: { chip: string }) {
  return (
    <span className={cn("rounded-full px-2 py-0.5 text-xs font-medium", COLORS[chip] ?? "bg-zinc-100 text-zinc-700")}>
      {chip}
    </span>
  );
}
