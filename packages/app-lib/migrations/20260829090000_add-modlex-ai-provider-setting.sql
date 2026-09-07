-- "auto" = обычная цепочка фолбэков Groq → z.ai → Cloudflare. Любое другое
-- значение форсирует конкретного провайдера в обход остальных — нужно для
-- изоляции багов конкретного провайдера при тестировании.
ALTER TABLE settings ADD COLUMN modlex_ai_provider TEXT NOT NULL DEFAULT 'auto';
