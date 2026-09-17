ALTER TABLE settings ADD COLUMN modlex_global_background_path TEXT;
ALTER TABLE settings ADD COLUMN modlex_global_background_opacity REAL NOT NULL DEFAULT 0.5;
ALTER TABLE settings ADD COLUMN modlex_global_background_blur_px REAL NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN modlex_global_background_animated BOOLEAN NOT NULL DEFAULT TRUE;
