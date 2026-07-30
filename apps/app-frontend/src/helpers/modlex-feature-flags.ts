// apps/app-frontend/src/helpers/modlex-feature-flags.ts
//
// Удалённый kill-switch для фич-флагов: тянет flags.json с того же GitHub Pages,
// что и news.json (см. modlex-github-news.ts), и подмешивает поверх локальных
// значений из feature-flags.ts. Если что-то сломалось после релиза (например,
// поменялось CurseForge API) — правите flags.json на сайте, и уже установленные
// билды подхватят отключение без новой сборки.
//
// Формат flags.json:
// { "flags": { "curseforge_platform": { "enabled": false, "locked": true, "message": "..." } } }
//
// В отличие от новостей, здесь всегда идёт живой запрос (файл крошечный, лишний
// трафик не ощутим) — кэш используется только как fallback, если сайт недоступен,
// а не как способ пропустить запрос. Это важно для kill-switch: свежесть важнее
// экономии одного HTTP-запроса.

import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

import { featureFlags, LOCAL_FEATURE_FLAGS, type FeatureFlags } from './feature-flags'

const FLAGS_URL = 'https://gussiak.github.io/flags.json'

const CACHE_KEY = 'modlex_flags_cache'
const POLL_INTERVAL_MS = 10 * 60 * 1000 // 10 минут — как часто перепроверяем, пока приложение открыто

interface CacheShape {
	fetchedAt: number
	flags: FeatureFlags
}

let pollTimer: ReturnType<typeof setInterval> | null = null

export async function refreshFeatureFlags(): Promise<void> {
	try {
		const response = await tauriFetch(FLAGS_URL, {
			headers: { Accept: 'application/json' },
		})

		if (!response.ok) {
			throw new Error(`flags.json ответил статусом ${response.status}`)
		}

		const data = (await response.json()) as { flags?: FeatureFlags }
		const remoteFlags = data.flags ?? {}

		writeCache({ fetchedAt: Date.now(), flags: remoteFlags })
		applyRemoteFlags(remoteFlags)
	} catch (err) {
		// Сайт недоступен (нет интернета, GitHub Pages лежит и т.п.) — используем
		// последний известный кэш как fallback, а не блокируем/ломаем приложение
		console.warn('Не удалось обновить feature-флаги ModLEX:', err)
		const cached = readCache()
		if (cached) {
			applyRemoteFlags(cached.flags)
		}
	}
}

/** Запускает периодическую перепроверку flags.json, пока приложение открыто. */
export function startFeatureFlagPolling() {
	if (pollTimer) return
	pollTimer = setInterval(refreshFeatureFlags, POLL_INTERVAL_MS)
}

export function stopFeatureFlagPolling() {
	if (pollTimer) {
		clearInterval(pollTimer)
		pollTimer = null
	}
}

function applyRemoteFlags(remoteFlags: FeatureFlags) {
	featureFlags.value = { ...LOCAL_FEATURE_FLAGS, ...remoteFlags }
}

function readCache(): CacheShape | null {
	try {
		const raw = localStorage.getItem(CACHE_KEY)
		return raw ? (JSON.parse(raw) as CacheShape) : null
	} catch {
		return null
	}
}

function writeCache(data: CacheShape) {
	localStorage.setItem(CACHE_KEY, JSON.stringify(data))
}
