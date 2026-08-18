// apps/app-frontend/src/helpers/modlex-ping.ts
//
// Минимальный обезличенный пинг: раз при старте и затем каждые
// PING_INTERVAL_MS, пока приложение открыто, отправляет { id } на PING_URL —
// id это тот же случайный UUID, что и modlex_tester_id (см.
// modlex-beta-channel.ts::ensureTesterId), никакой другой информации не
// передаётся. Сервер по этим id считает, сколько уникальных установок вообще
// открывали лаунчер и сколько сейчас активны (по свежести последнего пинга).
//
// Сбои сети/недоступность сервера намеренно проглатываются — статистика не
// критична для работы приложения, ничего не должно из-за неё сломаться.

import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

import { ensureTesterId } from '@/helpers/modlex-beta-channel'

const PING_URL = 'https://ping.modlex.deno.net/ping'

const PING_INTERVAL_MS = 90 * 1000 // 90 секунд

let pingTimer: ReturnType<typeof setInterval> | null = null

async function sendPing(): Promise<void> {
	try {
		const id = await ensureTesterId()
		await tauriFetch(PING_URL, {
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ id }),
		})
	} catch {
		// тихо игнорируем — см. комментарий выше файла
	}
}

/** Запускает обезличенный пинг: сразу один раз, затем по таймеру. */
export function startPing(): void {
	sendPing()
	if (pingTimer) return
	pingTimer = setInterval(sendPing, PING_INTERVAL_MS)
}

export function stopPing(): void {
	if (pingTimer) {
		clearInterval(pingTimer)
		pingTimer = null
	}
}
