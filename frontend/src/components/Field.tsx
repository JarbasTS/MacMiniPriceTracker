import type { InputHTMLAttributes } from "react";

import { cn } from "@/lib/utils";

type Props = InputHTMLAttributes<HTMLInputElement> & { label: string; name: string; error?: string; hint?: string };

/** Input com label e mensagem de erro ligadas por id (acessível a leitores de tela). */
export default function Field({ label, name, error, hint, className, ...input }: Props) {
  const errId = `${name}-error`;
  return (
    <label className={cn("flex flex-col gap-1 text-sm", className)}>
      <span className="font-medium text-zinc-700">{label}</span>
      <input
        name={name}
        aria-invalid={error ? true : undefined}
        aria-describedby={error ? errId : undefined}
        className={cn(
          "rounded-md border border-zinc-300 px-2.5 py-1.5 text-zinc-900 outline-none focus:border-zinc-900",
          error && "border-red-500",
        )}
        {...input}
      />
      {hint && !error && <span className="text-xs text-zinc-500">{hint}</span>}
      {error && (
        <span id={errId} className="text-xs text-red-600">
          {error}
        </span>
      )}
    </label>
  );
}
