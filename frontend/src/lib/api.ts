import type {
  ListingDetail,
  ListingSort,
  ListingView,
  NewStore,
  NewTarget,
  ScrapeReport,
  SearchTarget,
  StatsSummary,
  Store,
  StorePatch,
  TargetPatch,
} from "./types";

const API = process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080/api";

/** Erro da API com o status HTTP, para tratar casos como 409 (varredura em andamento). */
export class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API}${path}`, {
    ...init,
    // Content-Type só com corpo: evita preflight de CORS nos GETs.
    headers: init?.body ? { "Content-Type": "application/json" } : undefined,
    cache: "no-store",
  });
  if (res.status === 204) return undefined as T;
  const body = await res.json().catch(() => null);
  if (!res.ok) throw new ApiError(body?.error ?? `HTTP ${res.status}`, res.status);
  return body as T;
}

const json = (method: string, data: unknown): RequestInit => ({ method, body: JSON.stringify(data) });

export function getListings(filters: { chip_family?: string; store_id?: string; sort?: ListingSort } = {}) {
  const qs = new URLSearchParams(Object.entries(filters).filter(([, v]) => v) as [string, string][]);
  return request<ListingView[]>(`/listings${qs.size ? `?${qs}` : ""}`);
}

export const getListing = (id: string) => request<ListingDetail>(`/listings/${encodeURIComponent(id)}`);
export const getSummary = () => request<StatsSummary>("/stats/summary");

export const getStores = () => request<Store[]>("/stores");
export const createStore = (data: NewStore) => request<Store>("/stores", json("POST", data));
export const updateStore = (id: string, data: StorePatch) => request<Store>(`/stores/${id}`, json("PATCH", data));

export const getTargets = () => request<SearchTarget[]>("/targets");
export const createTarget = (data: NewTarget) => request<SearchTarget>("/targets", json("POST", data));
export const updateTarget = (id: string, data: TargetPatch) =>
  request<SearchTarget>(`/targets/${id}`, json("PATCH", data));
export const deleteTarget = (id: string) => request<void>(`/targets/${id}`, { method: "DELETE" });

/** Sem `wait`: o backend responde 202 e varre em background (409 se já houver uma varredura). */
export const triggerScrape = () => request<{ status: string } | ScrapeReport>("/scrape/trigger", { method: "POST" });
