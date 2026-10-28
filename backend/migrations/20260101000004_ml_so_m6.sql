-- Mercado Livre busca só o Mac mini M6 (o runner filtra os alvos por scraper_config.chip_families).
UPDATE stores SET scraper_config = '{"chip_families": ["M6"]}'::jsonb WHERE slug = 'mercado_livre';
