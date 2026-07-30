ALTER TABLE minecraft_users ADD COLUMN kind TEXT NOT NULL DEFAULT 'microsoft';

UPDATE minecraft_users SET kind = 'offline' WHERE access_token = 'null' AND refresh_token = 'null';
