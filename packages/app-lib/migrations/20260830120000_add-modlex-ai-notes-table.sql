-- Заметки ИИ-агента ModLEX — что поставил/изменил/выяснил, для контекста в
-- следующей сессии (агент сам решает, когда стоит записать). instance_id
-- NULL — заметка общего характера, не привязанная к конкретной сборке.
CREATE TABLE modlex_ai_notes (
	id INTEGER NOT NULL,
	instance_id TEXT NULL,
	content TEXT NOT NULL,
	created INTEGER NOT NULL,

	PRIMARY KEY (id AUTOINCREMENT),
	FOREIGN KEY (instance_id) REFERENCES instances(id) ON DELETE CASCADE
);
