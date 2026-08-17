ALTER TABLE settings ADD COLUMN modlex_vk_token TEXT;
ALTER TABLE settings ADD COLUMN modlex_vk_user_id INTEGER;
ALTER TABLE settings ADD COLUMN modlex_soundcloud_enabled BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE settings ADD COLUMN modlex_local_music_path TEXT;
ALTER TABLE settings ADD COLUMN modlex_music_default_source TEXT NOT NULL DEFAULT 'soundcloud';
