-- "playstation 5" no catálogo do ML só trazia jogos e acessórios.
UPDATE search_targets SET search_query = 'console playstation 5',
    excluded_keywords = excluded_keywords || ARRAY['nvme']
WHERE chip_family = 'PS5' AND NOT ('nvme' = ANY(excluded_keywords));

-- SSD "compatível com PS5" que entrou antes do filtro.
DELETE FROM listings l USING search_targets t
WHERE l.target_id = t.id AND t.chip_family = 'PS5' AND l.title ILIKE '%nvme%';

-- O catálogo do ML não tem o M6 ainda; sem chip_families a loja busca todos os alvos.
UPDATE stores SET scraper_config = '{}'::jsonb WHERE slug = 'mercado_livre';
