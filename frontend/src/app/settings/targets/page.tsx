"use client";

import { useEffect, useState, type FormEvent } from "react";
import { Pencil, Plus, Trash2 } from "lucide-react";
import { z } from "zod";

import ChipBadge from "@/components/ChipBadge";
import Field from "@/components/Field";
import { createTarget, deleteTarget, getTargets, updateTarget } from "@/lib/api";
import type { SearchTarget } from "@/lib/types";
import { CHIPS, formatBRL } from "@/lib/utils";

const csv = z.string().transform((s) =>
  s
    .split(",")
    .map((k) => k.trim())
    .filter(Boolean),
);

// Espelha a validação do backend (routes/targets.rs).
const schema = z
  .object({
    name: z.string().trim().min(1, "Obrigatório"),
    chip_family: z.string().trim().min(1, "Obrigatório"),
    search_query: z.string().trim().min(1, "Obrigatório"),
    required_keywords: csv.pipe(z.array(z.string()).min(1, "Informe ao menos uma palavra")),
    excluded_keywords: csv,
    min_price_brl: z.coerce.number("Número inválido").positive("Deve ser maior que zero"),
    max_price_brl: z.preprocess(
      (v) => (v === "" ? undefined : v),
      z.coerce.number("Número inválido").positive("Deve ser maior que zero").optional(),
    ),
    is_active: z.preprocess((v) => v === "on", z.boolean()),
  })
  .refine((d) => d.max_price_brl === undefined || d.max_price_brl >= d.min_price_brl, {
    message: "Deve ser maior ou igual ao mínimo",
    path: ["max_price_brl"],
  });

type Errors = Partial<Record<keyof z.infer<typeof schema>, string>>;

/** `null` = formulário fechado, `"new"` = criando, objeto = editando. */
type Editing = null | "new" | SearchTarget;

export default function TargetsPage() {
  const [targets, setTargets] = useState<SearchTarget[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [editing, setEditing] = useState<Editing>(null);
  const [confirmId, setConfirmId] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    getTargets()
      .then((data) => alive && setTargets(data))
      .catch((e: Error) => alive && setError(e.message));
    return () => {
      alive = false;
    };
  }, []);

  async function remove(id: string) {
    setError(null);
    try {
      await deleteTarget(id);
      setTargets((list) => list?.filter((t) => t.id !== id) ?? null);
    } catch (e) {
      setError((e as Error).message);
    } finally {
      setConfirmId(null);
    }
  }

  function saved(t: SearchTarget) {
    setTargets((list) => {
      const rest = (list ?? []).filter((x) => x.id !== t.id);
      return [...rest, t].sort((a, b) => a.name.localeCompare(b.name));
    });
    setEditing(null);
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">Alvos de busca</h1>
        <button
          type="button"
          onClick={() => setEditing("new")}
          className="inline-flex items-center gap-1.5 rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white hover:bg-zinc-800"
        >
          <Plus className="size-4" aria-hidden /> Novo alvo
        </button>
      </div>

      {error && (
        <p role="alert" className="rounded-md bg-red-50 px-3 py-2 text-sm text-red-700">
          {error}
        </p>
      )}

      {editing && (
        <TargetForm
          // key força remontar com os valores do alvo escolhido.
          key={editing === "new" ? "new" : editing.id}
          target={editing === "new" ? null : editing}
          onCancel={() => setEditing(null)}
          onSaved={saved}
        />
      )}

      {targets === null ? (
        !error && <p className="text-sm text-zinc-500">Carregando alvos…</p>
      ) : (
        <ul className="flex flex-col gap-2">
          {targets.map((t) => (
            <li key={t.id} className="flex flex-wrap items-center gap-3 rounded-lg border border-zinc-200 bg-white p-4">
              <div className="flex min-w-0 flex-1 flex-col gap-1">
                <div className="flex items-center gap-2">
                  <span className="font-medium">{t.name}</span>
                  <ChipBadge chip={t.chip_family} />
                  {!t.is_active && <span className="text-xs text-zinc-500">(inativo)</span>}
                </div>
                <p className="text-sm text-zinc-600">
                  “{t.search_query}” · mín. {formatBRL(t.min_price_brl)}
                  {t.max_price_brl && ` · máx. ${formatBRL(t.max_price_brl)}`}
                </p>
                <p className="truncate text-xs text-zinc-500">
                  Exige: {t.required_keywords.join(", ")} · Exclui {t.excluded_keywords.length} palavras
                </p>
              </div>
              {confirmId === t.id ? (
                <div className="flex items-center gap-2 text-sm" role="group" aria-label="Confirmar exclusão">
                  <span>Excluir “{t.name}”?</span>
                  <button
                    type="button"
                    onClick={() => remove(t.id)}
                    className="rounded-md bg-red-600 px-2.5 py-1 font-medium text-white hover:bg-red-700"
                  >
                    Sim, excluir
                  </button>
                  <button type="button" onClick={() => setConfirmId(null)} className="rounded-md px-2.5 py-1 hover:bg-zinc-100">
                    Não
                  </button>
                </div>
              ) : (
                <div className="flex gap-1">
                  <button
                    type="button"
                    onClick={() => setEditing(t)}
                    aria-label={`Editar ${t.name}`}
                    className="rounded-md p-2 text-zinc-600 hover:bg-zinc-100"
                  >
                    <Pencil className="size-4" aria-hidden />
                  </button>
                  <button
                    type="button"
                    onClick={() => setConfirmId(t.id)}
                    aria-label={`Excluir ${t.name}`}
                    className="rounded-md p-2 text-red-600 hover:bg-red-50"
                  >
                    <Trash2 className="size-4" aria-hidden />
                  </button>
                </div>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function TargetForm({
  target,
  onCancel,
  onSaved,
}: {
  target: SearchTarget | null;
  onCancel: () => void;
  onSaved: (t: SearchTarget) => void;
}) {
  const [errors, setErrors] = useState<Errors>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setServerError(null);
    const parsed = schema.safeParse(Object.fromEntries(new FormData(e.currentTarget)));
    if (!parsed.success) {
      const fields = z.flattenError(parsed.error).fieldErrors;
      setErrors(Object.fromEntries(Object.entries(fields).map(([k, v]) => [k, v?.[0]])) as Errors);
      return;
    }
    setErrors({});
    const data = parsed.data;
    setSaving(true);
    try {
      const result = target
        ? await updateTarget(target.id, data)
        : // Na criação, lista vazia = deixa o backend aplicar a lista padrão anti-acessórios.
          await createTarget({ ...data, excluded_keywords: data.excluded_keywords.length ? data.excluded_keywords : undefined });
      onSaved(result);
    } catch (err) {
      setServerError((err as Error).message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <form
      onSubmit={handleSubmit}
      noValidate
      className="grid grid-cols-1 gap-3 rounded-lg border border-zinc-300 bg-white p-5 sm:grid-cols-2"
      aria-label={target ? `Editar ${target.name}` : "Novo alvo"}
    >
      <h2 className="font-semibold sm:col-span-2">{target ? `Editar “${target.name}”` : "Novo alvo"}</h2>
      <Field label="Nome" name="name" defaultValue={target?.name} error={errors.name} autoFocus />
      <Field
        label="Família do chip"
        name="chip_family"
        defaultValue={target?.chip_family}
        error={errors.chip_family}
        list="chip-options"
      />
      <datalist id="chip-options">
        {CHIPS.map((c) => (
          <option key={c} value={c} />
        ))}
      </datalist>
      <Field
        label="Termo de busca"
        name="search_query"
        defaultValue={target?.search_query}
        error={errors.search_query}
        className="sm:col-span-2"
        placeholder="mac mini m6"
      />
      <Field
        label="Palavras obrigatórias"
        name="required_keywords"
        defaultValue={target?.required_keywords.join(", ") ?? "mac, mini"}
        error={errors.required_keywords}
        hint="Separadas por vírgula"
        className="sm:col-span-2"
      />
      <Field
        label="Palavras excluídas"
        name="excluded_keywords"
        defaultValue={target?.excluded_keywords.join(", ")}
        error={errors.excluded_keywords}
        hint={target ? "Separadas por vírgula" : "Separadas por vírgula; vazio = lista padrão anti-acessórios"}
        className="sm:col-span-2"
      />
      <Field
        label="Preço mínimo (R$)"
        name="min_price_brl"
        type="number"
        step="0.01"
        min="0"
        defaultValue={target?.min_price_brl}
        error={errors.min_price_brl}
      />
      <Field
        label="Preço máximo (R$, opcional)"
        name="max_price_brl"
        type="number"
        step="0.01"
        min="0"
        defaultValue={target?.max_price_brl ?? ""}
        error={errors.max_price_brl}
        hint={target?.max_price_brl ? "Deixar vazio mantém o valor atual" : undefined}
      />
      <label className="flex items-center gap-2 text-sm sm:col-span-2">
        <input type="checkbox" name="is_active" defaultChecked={target?.is_active ?? true} /> Ativo
      </label>
      {serverError && (
        <p role="alert" className="rounded-md bg-red-50 px-3 py-2 text-sm text-red-700 sm:col-span-2">
          {serverError}
        </p>
      )}
      <div className="flex justify-end gap-2 sm:col-span-2">
        <button type="button" onClick={onCancel} className="rounded-md px-3 py-2 text-sm hover:bg-zinc-100">
          Cancelar
        </button>
        <button
          type="submit"
          disabled={saving}
          className="rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white hover:bg-zinc-800 disabled:opacity-60"
        >
          {saving ? "Salvando…" : "Salvar alvo"}
        </button>
      </div>
    </form>
  );
}
