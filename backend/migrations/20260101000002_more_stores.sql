-- KaBuM: seletores CSS do seed não funcionam (produtos carregam via JS); usa a API JSON da busca.
UPDATE stores SET
    slug = 'kabum',
    scraper_type = 'kabum_api',
    name = 'KaBuM!',
    search_url_template = 'https://servicespub.prod.api.aws.grupokabum.com.br/catalog/v2/products?query={query}&page_number=1&page_size=60&sort=most_searched',
    scraper_config = '{}'::jsonb
WHERE slug = 'kabum_custom';

-- Mercado Livre: API exige app aprovado (403 mesmo com OAuth) e o HTML tem anti-bot.
UPDATE stores SET is_active = false WHERE slug = 'mercado_livre';

INSERT INTO stores (slug, scraper_type, name, base_url, search_url_template) VALUES
('buscape', 'buscape', 'Buscapé', 'https://www.buscape.com.br', 'https://www.buscape.com.br/search?q={query}'),
('apple_br', 'apple_br', 'Apple Store', 'https://www.apple.com/br', 'https://www.apple.com/br/shop/buy-mac/mac-mini');
