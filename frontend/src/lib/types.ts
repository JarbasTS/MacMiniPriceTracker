/** Decimal do backend: chega como string ("10700.00") para não perder precisão. */
export type Money = string;

export interface Store {
  id: string;
  slug: string;
  scraper_type: string;
  name: string;
  base_url: string;
  search_url_template: string;
  is_active: boolean;
  scrape_interval_minutes: number;
  scraper_config: Record<string, unknown>;
  created_at: string;
}

export interface SearchTarget {
  id: string;
  name: string;
  chip_family: string;
  search_query: string;
  required_keywords: string[];
  excluded_keywords: string[];
  min_price_brl: Money;
  max_price_brl: Money | null;
  is_active: boolean;
  created_at: string;
}

export interface Listing {
  id: string;
  store_id: string;
  target_id: string;
  external_id: string;
  title: string;
  url: string;
  image_url: string | null;
  seller_name: string | null;
  current_price: Money;
  installment_price: Money | null;
  lowest_price: Money;
  highest_price: Money;
  in_stock: boolean;
  first_seen_at: string;
  last_seen_at: string;
}

export interface ListingView extends Listing {
  store_name: string;
  store_slug: string;
  target_name: string;
  chip_family: string;
}

export interface PriceHistory {
  id: number;
  listing_id: string;
  price_cash: Money;
  price_installments: Money | null;
  captured_at: string;
}

export type ListingDetail = ListingView & { history: PriceHistory[] };

export interface ChipMin {
  chip_family: string;
  min_price: Money;
  store_name: string;
  listing_id: string;
}

export interface StatsSummary {
  by_chip: ChipMin[];
  total_listings: number;
  last_scrape_at: string | null;
}

export interface ScrapeReport {
  stores: number;
  found: number;
  accepted: number;
  errors: number;
}

export type ListingSort = "price_asc" | "price_desc" | "recent";

export interface NewStore {
  slug: string;
  scraper_type: string;
  name: string;
  base_url: string;
  search_url_template: string;
  is_active?: boolean;
  scrape_interval_minutes?: number;
  scraper_config?: Record<string, unknown>;
}

export type StorePatch = Partial<
  Pick<Store, "is_active" | "name" | "search_url_template" | "scrape_interval_minutes" | "scraper_config">
>;

export interface NewTarget {
  name: string;
  chip_family: string;
  search_query: string;
  required_keywords: string[];
  /** Ausente = lista padrão anti-acessórios do backend. */
  excluded_keywords?: string[];
  min_price_brl: number;
  max_price_brl?: number;
  is_active?: boolean;
}

export type TargetPatch = Partial<NewTarget>;
