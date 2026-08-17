// apps/app-frontend/src/helpers/modlex-beta-channel.ts
//
// Тестовые коды для бета-канала. Публикуется только SHA-256 хэш кода (не сам
// код) в JSON на gussiak.github.io — так же, как flags.json/news.json.
//
// Анти-утечка: у каждого хэша в JSON — список одобренных tester-ID
// (случайный UUID, сгенерированный ЛОКАЛЬНО при первом вводе валидного кода
// и сохранённый в settings.modlex_tester_id — переиспользуется при каждой
// следующей проверке, повторно вводить код не нужно). Ввод правильного кода
// сам по себе доступ не даёт: пока конкретный tester-ID не появится в списке
// одобренных для этого хэша (вручную одобряется через правку JSON), доступ
// заблокирован — то есть слитый код без отдельного одобрения владельца
// каждого устройства бесполезен.
//
// Формат beta-testers.json:
// { "<sha256 код в hex>": ["<одобренный tester-id>", ...] }

import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

import { get as getSettings, set as setSettings } from '@/helpers/settings'

const TESTERS_URL = 'https://gussiak.github.io/beta-testers.json'

type TestersMap = Record<string, string[]>

export type CodeCheckResult =
	| { status: 'invalid' }
	| { status: 'pending'; testerId: string }
	| { status: 'approved'; testerId: string }

async function sha256Hex(value: string): Promise<string> {
	const data = new TextEncoder().encode(value.trim())
	const digest = await crypto.subtle.digest('SHA-256', data)
	return Array.from(new Uint8Array(digest))
		.map((b) => b.toString(16).padStart(2, '0'))
		.join('')
}

async function fetchTesters(): Promise<TestersMap> {
	const response = await tauriFetch(`${TESTERS_URL}?t=${Date.now()}`, {
		headers: { Accept: 'application/json' },
	})
	if (!response.ok) {
		throw new Error(`beta-testers.json ответил статусом ${response.status}`)
	}
	return (await response.json()) as TestersMap
}

/** Возвращает существующий tester-id из настроек, либо создаёт и сохраняет новый. */
export async function ensureTesterId(): Promise<string> {
	const settings = await getSettings()
	if (settings.modlex_tester_id) {
		return settings.modlex_tester_id
	}
	const testerId = crypto.randomUUID()
	settings.modlex_tester_id = testerId
	await setSettings(settings)
	return testerId
}

/**
 * Проверяет тестовый код. Не включает бета-канал сама — вызывающий код
 * решает, что делать по результату (approved / pending / invalid).
 */
export async function checkTesterCode(code: string): Promise<CodeCheckResult> {
	const [hash, testers, testerId] = await Promise.all([
		sha256Hex(code),
		fetchTesters(),
		ensureTesterId(),
	])

	const approvedIds = testers[hash]
	if (!approvedIds) {
		return { status: 'invalid' }
	}
	if (approvedIds.includes(testerId)) {
		return { status: 'approved', testerId }
	}
	return { status: 'pending', testerId }
}
