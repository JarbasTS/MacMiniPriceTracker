-- Tokens OAuth por provedor. O refresh token do Mercado Livre é de uso único e muda a cada renovação,
-- por isso vive aqui e não no .env (o .env só semeia a primeira linha).
CREATE TABLE oauth_tokens (
    provider VARCHAR(30) PRIMARY KEY,       -- 'mercado_livre'
    refresh_token TEXT NOT NULL,
    access_token TEXT,
    expires_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Mercado Livre: /products/search (catálogo) responde com token; a busca comum e /items seguem 403.
UPDATE stores SET is_active = true WHERE slug = 'mercado_livre';
