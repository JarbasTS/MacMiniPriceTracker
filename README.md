# Mac mini Price Tracker

Self-hosted dashboard that watches Brazilian e-commerce sites for **Mac mini M5 / M5 Pro / M6** listings, filters out accessories and wrong models, and keeps the price history of each one.

<!-- Add a screenshot at docs/screenshot.png and uncomment:
![Dashboard](docs/screenshot.png)
-->

## Features

- **Multi-store scraping:** KaBuM (JSON API), Amazon BR, Apple Store BR, Buscapé, and Mercado Livre (catalog API with OAuth, HTML fallback).
- **Stores configurable at runtime:** add a new shop from the UI with CSS selectors stored as JSONB. No recompiling.
- **Smart filtering:** a sanitizer rejects cases, cables, "empty box" listings and wrong chips (an M5 Pro won't show up under the M5 target).
- **Price history:** every scrape stores a price point, and the detail page charts it.
- **Background scheduler:** a cron worker scrapes each store on its own interval. A lock prevents overlapping runs. There is also a "refresh now" button.
- **One-command deploy:** Docker Compose with multi-stage builds (`cargo-chef` for Rust, `standalone` for Next.js), ready for ARM64.

## Stack

| Layer | Tech |
|---|---|
| Backend | Rust · axum · tokio · sqlx · wreq (browser-like TLS fingerprint) · scraper · tokio-cron-scheduler |
| Database | PostgreSQL 16 |
| Frontend | Next.js (App Router) · TypeScript · Tailwind CSS · recharts · zod |
| Infra | Docker Compose · Cloudflare Tunnel + Access |

## Architecture

```
 Browser ──▶ Next.js frontend ──▶ /api (axum) ──▶ PostgreSQL
                                     ▲
                        scheduler ───┘ every 5 min: for each store due
                           │
                           ▼
          StoreScraper trait ── KaBuM API · Amazon · Apple · Buscapé · ML · Dynamic CSS
                           │
                           ▼
                 sanitizer (keywords / chip rules) ──▶ listings + price_history
```

## Getting started

Requirements: Docker with Compose.

```bash
git clone https://github.com/JarbasTS/MacMiniPriceTracker.git
cd MacMiniPriceTracker
cp .env.example .env
docker compose up --build
```

- Dashboard: http://localhost:3000
- API health: http://localhost:8080/api/health

Migrations run automatically on startup and seed the default stores and search targets.

## Project structure

```
backend/     Rust API, scrapers, scheduler, SQL migrations
frontend/    Next.js dashboard (listings, detail chart, stores & targets settings)
docs/        deploy guide
```

## API

| Method | Route | Description |
|---|---|---|
| GET | `/api/health` | Health check |
| GET | `/api/listings` | Current listings |
| GET | `/api/listings/:id` | Listing with price history |
| GET | `/api/stats/summary` | Dashboard summary |
| GET · POST / PATCH | `/api/stores` · `/api/stores/:id` | List, create and edit stores |
| GET · POST / PATCH · DELETE | `/api/targets` · `/api/targets/:id` | Manage search targets |
| POST | `/api/scrape/trigger` | Trigger a scrape now |
| GET | `/api/scrape/status` | `{ "running": bool }`: is a scrape in progress |

> The API has no built-in auth. In production it sits behind Cloudflare Access. See [docs/deploy.md](docs/deploy.md).

## Notes

- Mercado Livre: the regular search API (`/sites/MLB/search`) returns 403 for non-certified apps, but the catalog (`/products/search` + `/products/{id}/items`) works with an OAuth token. The rotating refresh token is stored in the `oauth_tokens` table, seeded once from `ML_REFRESH_TOKEN`.
- Datacenter IPs get blocked more often than residential ones. Scrape intervals are configurable per store.

## License

[MIT](LICENSE)
