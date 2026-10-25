"use client";

import { useEffect, useState, type FormEvent } from "react";
import { X } from "lucide-react";
import { z } from "zod";

import Field from "@/components/Field";
import { createStore } from "@/lib/api";
import type { Store } from "@/lib/types";

const required = z.string().trim().min(1, "Obrigatório");
const optional = z
  .string()
  .trim()
  .transform((s) => s || undefined);

const schema = z.object({
  name: required,
  slug: z.string().trim().regex(/^[a-z0-9_]+$/, "Use só minúsculas, números e _"),
  base_url: z.url("URL inválida"),
  search_url_template: z.url("URL inválida").refine((s) => s.includes("{query}"), "Deve conter {query}"),
  scrape_interval_minutes: z.coerce.number().int("Número inteiro").min(1, "Mínimo 1"),
  item_selector: required,
  title_selector: required,
  price_selector: required,
  url_selector: required,
  image_selector: optional,
  external_id_attr: optional,
});

type Errors = Partial<Record<keyof z.infer<typeof schema>, string>>;

export default function StoreFormModal({ onClose, onCreated }: { onClose: () => void; onCreated: (s: Store) => void }) {
  const [errors, setErrors] = useState<Errors>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

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
    const { item_selector, title_selector, price_selector, url_selector, image_selector, external_id_attr, ...store } =
      parsed.data;
    setSaving(true);
    try {
      const created = await createStore({
        ...store,
        scraper_type: "dynamic_css",
        scraper_config: { item_selector, title_selector, price_selector, url_selector, image_selector, external_id_attr },
      });
      onCreated(created);
    } catch (err) {
      // 400 do backend (ex.: seletor CSS inválido, slug duplicado) aparece dentro do modal.
      setServerError((err as Error).message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4" onClick={onClose}>
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby="store-modal-title"
        className="max-h-full w-full max-w-2xl overflow-y-auto rounded-lg bg-white p-6 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-center justify-between">
          <h2 id="store-modal-title" className="text-lg font-semibold text-zinc-900">
            Adicionar Nova Loja
          </h2>
          <button type="button" onClick={onClose} aria-label="Fechar" className="rounded p-1 hover:bg-zinc-100">
            <X className="size-5" aria-hidden />
          </button>
        </div>
        <form onSubmit={handleSubmit} noValidate className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <p className="text-sm text-zinc-600 sm:col-span-2">
            Tipo: <code className="rounded bg-zinc-100 px-1">dynamic_css</code> — o scraper genérico lê a página com os
            seletores CSS abaixo.
          </p>
          <Field label="Nome" name="name" error={errors.name} autoFocus />
          <Field label="Slug" name="slug" error={errors.slug} placeholder="minha_loja" />
          <Field label="URL base" name="base_url" error={errors.base_url} placeholder="https://www.loja.com.br" />
          <Field
            label="Intervalo (min)"
            name="scrape_interval_minutes"
            type="number"
            min={1}
            defaultValue={30}
            error={errors.scrape_interval_minutes}
          />
          <Field
            label="URL de busca"
            name="search_url_template"
            error={errors.search_url_template}
            placeholder="https://www.loja.com.br/busca/{query}"
            className="sm:col-span-2"
          />
          <Field label="item_selector" name="item_selector" error={errors.item_selector} placeholder="article.product" />
          <Field label="title_selector" name="title_selector" error={errors.title_selector} placeholder="h2.name" />
          <Field label="price_selector" name="price_selector" error={errors.price_selector} placeholder="span.price" />
          <Field label="url_selector" name="url_selector" error={errors.url_selector} placeholder="a.link" />
          <Field label="image_selector (opcional)" name="image_selector" error={errors.image_selector} />
          <Field
            label="external_id_attr (opcional)"
            name="external_id_attr"
            error={errors.external_id_attr}
            hint="Atributo do item com o id; vazio = usa a URL"
          />
          {serverError && (
            <p role="alert" className="rounded-md bg-red-50 px-3 py-2 text-sm text-red-700 sm:col-span-2">
              {serverError}
            </p>
          )}
          <div className="flex justify-end gap-2 sm:col-span-2">
            <button type="button" onClick={onClose} className="rounded-md px-3 py-2 text-sm hover:bg-zinc-100">
              Cancelar
            </button>
            <button
              type="submit"
              disabled={saving}
              className="rounded-md bg-zinc-900 px-3 py-2 text-sm font-medium text-white hover:bg-zinc-800 disabled:opacity-60"
            >
              {saving ? "Salvando…" : "Salvar loja"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
