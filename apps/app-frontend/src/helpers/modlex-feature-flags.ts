// apps/app-frontend/src/helpers/modlex-feature-flags.ts
//
// Удалённый kill-switch для фич-флагов: тянет flags.json с того же GitHub Pages,
// что и news.json (см. modlex-github-news.ts), и подмешивает поверх локальных
// значений из feature-flags.ts. Если что-то сломалось после релиза (например,
// поменялось CurseForge API) — правите flags.json на сайте, и уже установленные
// билды подхватят отключение без новой сборки.
//
// Формат flags.json:
// { "flags": { "curseforge_platform_v2": { "enabled": false, "locked": true, "message": "..." } } }
//
// Версионирование ключей: если смысл "enabled" для какой-то фичи меняется
// настолько, что старая и новая версия лаунчера поняли бы его по-разному
// (например, старый код умеет только черновик функции, а новый — уже
// реальную рабочую версию) — не переиспользуйте старое имя ключа, заводите
// новое (см. curseforge_platform → curseforge_platform_v2 в feature-flags.ts
// как пример и объяснение). Старый ключ в этом случае остаётся в самом
// flags.json на GitHub НАВСЕГДА выключенным (false) — ради всё ещё
// существующих старых установленных копий лаунчера, которые его читают.
//
// В отличие от новостей, здесь всегда идёт живой запрос (файл крошечный, лишний
// трафик не ощутим) — кэш используется только как fallback, если сайт недоступен,
// а не как способ пропустить запрос. Это важно для kill-switch: свежесть важнее
// экономии одного HTTP-запроса.
//
// Принудительные обновления (добавлено 2026-09-06): тот же flags.json несёт
// необязательное поле верхнего уровня "update" — {"min_required_version": "1.4.1",
// "message": "..."}. Если версия ЭТОГО запущенного лаунчера ниже указанной,
// сразу запускается проверка-и-автоустановка обновления (тот же путь, что уже
// использует переключение бета-канала в настройках — см. app-update.ts
// requestImmediateUpdateCheck), а не пассивное напоминание раз в 24 часа.
// Это НЕ спасает от битого самого flags.json (если файл не парсится, это поле
// тоже не будет прочитано) — только от случая "мы выкатили реально плохую
// версию и её надо снять с рук как можно быстрее".

import { getVersion } from '@tauri-apps/api/app'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

import { requestImmediateUpdateCheck } from '@/providers/app-update'

import { type FeatureFlags, featureFlags, LOCAL_FEATURE_FLAGS } from './feature-flags'

const FLAGS_URL = 'https://gussiak.github.io/flags.json'

const CACHE_KEY = 'modlex_flags_cache'
const POLL_INTERVAL_MS = 10 * 60 * 1000 // 10 минут — как часто перепроверяем, пока приложение открыто

interface ForcedUpdateInfo {
	min_required_version?: string
	message?: string
}

interface CacheShape {
	fetchedAt: number
	flags: FeatureFlags
	update?: ForcedUpdateInfo
}

let pollTimer: ReturnType<typeof setInterval> | null = null
let forcedUpdateInFlight = false

/** Простое сравнение версий вида "1.4.10" — не общий semver (пре-релизы и
 * т.п. не нужны, у лаунчера версии всегда major.minor.patch). */
function compareVersions(a: string, b: string): number {
	const partsA = a.split('.').map((n) => Number.parseInt(n, 10) || 0)
	const partsB = b.split('.').map((n) => Number.parseInt(n, 10) || 0)
	const len = Math.max(partsA.length, partsB.length)
	for (let i = 0; i < len; i++) {
		const diff = (partsA[i] ?? 0) - (partsB[i] ?? 0)
		if (diff !== 0) return diff
	}
	return 0
}

async function checkForcedUpdate(update: ForcedUpdateInfo | undefined): Promise<void> {
	if (!update?.min_required_version || forcedUpdateInFlight) return

	const currentVersion = await getVersion()
	if (compareVersions(currentVersion, update.min_required_version) >= 0) return

	console.warn(
		`Версия ${currentVersion} ниже обязательной ${update.min_required_version} — запускаю принудительное обновление.`,
		update.message ?? '',
	)
	forcedUpdateInFlight = true
	try {
		await requestImmediateUpdateCheck()
	} catch (err) {
		console.warn('Не удалось выполнить принудительное обновление:', err)
	} finally {
		// Если апдейт реально нашёлся и установился, installUpdate() уже закрыл
		// приложение — этот сброс имеет значение только если обновления на
		// текущем канале ещё нет (тогда следующий опрос flags.json попробует
		// снова, что и нужно для срочного патча).
		forcedUpdateInFlight = false
	}
}

export async function refreshFeatureFlags(): Promise<void> {
	try {
		const response = await tauriFetch(FLAGS_URL, {
			headers: { Accept: 'application/json' },
		})

		if (!response.ok) {
			throw new Error(`flags.json ответил статусом ${response.status}`)
		}

		const data = (await response.json()) as { flags?: FeatureFlags; update?: ForcedUpdateInfo }
		const remoteFlags = data.flags ?? {}

		writeCache({ fetchedAt: Date.now(), flags: remoteFlags, update: data.update })
		applyRemoteFlags(remoteFlags)
		await checkForcedUpdate(data.update)
	} catch (err) {
		// Сайт недоступен (нет интернета, GitHub Pages лежит и т.п.) — используем
		// последний известный кэш как fallback, а не блокируем/ломаем приложение
		console.warn('Не удалось обновить feature-флаги ModLEX:', err)
		const cached = readCache()
		if (cached) {
			applyRemoteFlags(cached.flags)
			await checkForcedUpdate(cached.update)
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
