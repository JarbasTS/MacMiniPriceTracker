CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- 1. Tabela de Lojas (Suporta scrapers nativos e scrapers dinâmicos via JSONB)
CREATE TABLE stores (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug VARCHAR(50) UNIQUE NOT NULL,       -- 'mercado_livre', 'amazon_br', ou slug customizado
    scraper_type VARCHAR(30) NOT NULL,      -- 'mercado_livre', 'amazon_br', 'dynamic_css'
    name VARCHAR(100) NOT NULL,
    base_url TEXT NOT NULL,
    search_url_template TEXT NOT NULL,      -- Ex: 'https://lista.mercadolivre.com.br/{query}'
    is_active BOOLEAN NOT NULL DEFAULT true,
    scrape_interval_minutes INT NOT NULL DEFAULT 30,
    scraper_config JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Alvos de Busca (Mac mini M5, M5 Pro, M6)
CREATE TABLE search_targets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    chip_family VARCHAR(20) NOT NULL,       -- 'M5', 'M5 Pro', 'M6'
    search_query VARCHAR(255) NOT NULL,
    required_keywords TEXT[] NOT NULL DEFAULT ARRAY['mac', 'mini'],
    excluded_keywords TEXT[] NOT NULL DEFAULT ARRAY[
        'hub', 'dock', 'suporte', 'support', 'stand', 'case', 'capa',
        'adesivo', 'skin', 'cabo', 'fonte', 'carregador', 'caixa vazia',
        'defeito', 'pecas', 'peças', 'm1', 'm2', 'm3', 'm4', 'intel', 'i5', 'i7'
    ],
    min_price_brl NUMERIC(10, 2) NOT NULL DEFAULT 3500.00,
    max_price_brl NUMERIC(10, 2),
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 3. Anúncios Encontrados (Listings)
CREATE TABLE listings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    store_id UUID NOT NULL REFERENCES stores(id) ON DELETE CASCADE,
    target_id UUID NOT NULL REFERENCES search_targets(id) ON DELETE CASCADE,
    external_id VARCHAR(255) NOT NULL,
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    image_url TEXT,
    seller_name VARCHAR(150),
    current_price NUMERIC(10, 2) NOT NULL,
    installment_price NUMERIC(10, 2),
    lowest_price NUMERIC(10, 2) NOT NULL,
    highest_price NUMERIC(10, 2) NOT NULL,
    in_stock BOOLEAN NOT NULL DEFAULT true,
    first_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(store_id, external_id)
);

-- 4. Histórico de Preços (Série Temporal)
CREATE TABLE price_history (
    id BIGSERIAL PRIMARY KEY,
    listing_id UUID NOT NULL REFERENCES listings(id) ON DELETE CASCADE,
    price_cash NUMERIC(10, 2) NOT NULL,
    price_installments NUMERIC(10, 2),
    captured_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_listings_target_price ON listings(target_id, current_price ASC);
CREATE INDEX idx_listings_store ON listings(store_id);
CREATE INDEX idx_price_history_listing_time ON price_history(listing_id, captured_at DESC);

-- Seed Inicial de Lojas
INSERT INTO stores (slug, scraper_type, name, base_url, search_url_template, scraper_config) VALUES
(
    'mercado_livre',
    'mercado_livre',
    'Mercado Livre',
    'https://www.mercadolivre.com.br',
    'https://lista.mercadolivre.com.br/{query}_NoIndex_True',
    '{"use_api_fallback": true}'::jsonb
),
(
    'amazon_br',
    'amazon_br',
    'Amazon Brasil',
    'https://www.amazon.com.br',
    'https://www.amazon.com.br/s?k={query}',
    '{}'::jsonb
),
(
    'kabum_custom',
    'dynamic_css',
    'KaBuM! (Exemplo Dinâmico)',
    'https://www.kabum.com.br',
    'https://www.kabum.com.br/busca/{query}',
    '{
        "item_selector": "article.productCard",
        "title_selector": "span.nameCard",
        "price_selector": "span.priceCard",
        "url_selector": "a.productLink",
        "image_selector": "img.imageCard",
        "external_id_attr": "data-smarthintproductid"
    }'::jsonb
);

-- Seed Inicial de Alvos (Mac mini M5, M5 Pro e M6)
INSERT INTO search_targets (name, chip_family, search_query, required_keywords, min_price_brl) VALUES
('Mac mini M5 (Base)', 'M5', 'mac mini m5', ARRAY['mac', 'mini', 'm5'], 3800.00),
('Mac mini M5 Pro', 'M5 Pro', 'mac mini m5 pro', ARRAY['mac', 'mini', 'm5', 'pro'], 6500.00),
('Mac mini M6', 'M6', 'mac mini m6', ARRAY['mac', 'mini', 'm6'], 4200.00);
