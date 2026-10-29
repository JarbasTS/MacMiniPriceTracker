-- PS5 (qualquer modelo: base, digital, Pro). Jogos e controles avulsos ficam abaixo do mínimo;
-- 'controle', 'jogo' e 'midia' não são excluídos porque aparecem em kits de console.
INSERT INTO search_targets (name, chip_family, search_query, required_keywords, excluded_keywords, min_price_brl) VALUES
('PlayStation 5', 'PS5', 'playstation 5', ARRAY['ps5|playstation 5|playstation5'],
 ARRAY['hub', 'dock', 'suporte', 'stand', 'case', 'capa', 'adesivo', 'skin', 'cabo', 'fonte', 'carregador',
       'carregamento', 'cooler', 'defeito', 'pecas', 'peças', 'caixa vazia', 'portal', 'vr', 'vr2'], 2000.00);

-- Pro/Max saíram do código (sanitizer) e viram dados: o alvo PS5 aceita o PS5 Pro.
UPDATE search_targets SET excluded_keywords = excluded_keywords || ARRAY['pro', 'max']
WHERE chip_family IN ('M5', 'M6') AND NOT ('pro' = ANY(excluded_keywords));

UPDATE stores SET scraper_config = '{"chip_families": ["M6", "PS5"]}'::jsonb WHERE slug = 'mercado_livre';
-- A Apple Store só vende Mac; evita uma requisição inútil por alvo.
UPDATE stores SET scraper_config = '{"chip_families": ["M5", "M5 Pro", "M6"]}'::jsonb WHERE slug = 'apple_br';
