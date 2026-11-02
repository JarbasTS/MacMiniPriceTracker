-- M5 base sai (cascade apaga listings e price_history); M5 Pro fica.
DELETE FROM search_targets WHERE chip_family = 'M5';

-- Lista padrão de exclusão, trocando m4 por m5/m6, mais pro/max (como na 000005).
INSERT INTO search_targets (name, chip_family, search_query, required_keywords, excluded_keywords, min_price_brl) VALUES
('Mac mini M4', 'M4', 'mac mini m4', ARRAY['mac', 'mini', 'm4'],
 ARRAY['hub', 'dock', 'suporte', 'support', 'stand', 'case', 'capa', 'adesivo', 'skin', 'cabo', 'fonte', 'carregador',
       'caixa vazia', 'defeito', 'pecas', 'peças', 'm1', 'm2', 'm3', 'm5', 'm6', 'intel', 'i5', 'i7', 'pro', 'max'], 3500.00);

UPDATE stores SET scraper_config = '{"chip_families": ["M4", "M5 Pro", "M6"]}'::jsonb WHERE slug = 'apple_br';

-- Preço buscado de hora em hora em todas as lojas.
ALTER TABLE stores ALTER COLUMN scrape_interval_minutes SET DEFAULT 60;
UPDATE stores SET scrape_interval_minutes = 60;
