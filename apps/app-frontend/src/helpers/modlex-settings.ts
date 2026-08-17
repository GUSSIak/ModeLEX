// apps/app-frontend/src/helpers/modlex-settings.ts

import { computed, ref, watch } from 'vue'

import { useFeatureFlag } from './feature-flags'

const STORAGE_KEYS = {
	hideServers: 'modlex_hide_servers',
	newsSource: 'modlex_news_source',
	enableModrinth: 'modlex_enable_modrinth',
	enableCurseForge: 'modlex_enable_curseforge',
	hideMusicTab: 'modlex_hide_music_tab',
	hideMultiLaunch: 'modlex_hide_multi_launch',
} as const

export type NewsSource = 'github' | 'modrinth' | 'off'

function readString(key: string, fallback: string): string {
	const raw = localStorage.getItem(key)
	return raw === null ? fallback : raw
}

function readBool(key: string, fallback: boolean): boolean {
	const raw = localStorage.getItem(key)
	return raw === null ? fallback : raw === 'true'
}

// ── Внешний вид ────────────────────────────────────────────────────────────
export const modlexHideServers = ref(readBool(STORAGE_KEYS.hideServers, false))

// ── Музыка ─────────────────────────────────────────────────────────────────
export const modlexHideMusicTab = ref(readBool(STORAGE_KEYS.hideMusicTab, false))

// ── Мульти-запуск ──────────────────────────────────────────────────────────
export const modlexHideMultiLaunch = ref(readBool(STORAGE_KEYS.hideMultiLaunch, false))

// ── Новости ────────────────────────────────────────────────────────────────
export const modlexNewsSource = ref<NewsSource>(
	readString(STORAGE_KEYS.newsSource, 'github') as NewsSource,
)

export const modlexUseGithubNews = computed(() => modlexNewsSource.value === 'github')
export const modlexHideNews = computed(() => modlexNewsSource.value === 'off')
export const modlexUseModrinthNews = computed(() => modlexNewsSource.value === 'modrinth')

// ── Платформы поиска ───────────────────────────────────────────────────────
export const modlexEnableModrinth = ref(readBool(STORAGE_KEYS.enableModrinth, true))
export const modlexEnableCurseForge = ref(readBool(STORAGE_KEYS.enableCurseForge, true))

/**
 * Какие платформы реально доступны (хотя бы одна должна быть включена).
 * useFeatureFlag вызывается лениво внутри computed (а не на верхнем уровне
 * модуля), потому что он использует Pinia (useTheming) — на верхнем уровне
 * модуля Pinia может быть ещё не подключена в зависимости от порядка импортов.
 */
export const availablePlatforms = computed<Array<'modrinth' | 'curseforge'>>(() => {
	const list: Array<'modrinth' | 'curseforge'> = []
	if (modlexEnableModrinth.value) list.push('modrinth')
	// Фича-флаг может отключить CurseForge независимо от выбора пользователя
	// (useFeatureFlag сам учитывает devMode — см. helpers/feature-flags.ts)
	if (modlexEnableCurseForge.value && useFeatureFlag('curseforge_platform').enabled.value) {
		list.push('curseforge')
	}
	// хотя бы одна всегда есть — если обе выкл, возвращаем modrinth как fallback
	return list.length > 0 ? list : ['modrinth']
})

// ── Watchers ───────────────────────────────────────────────────────────────
function broadcast() {
	window.dispatchEvent(
		new CustomEvent('modlex-settings-changed', {
			detail: {
				hideServers: modlexHideServers.value,
				newsSource: modlexNewsSource.value,
				enableModrinth: modlexEnableModrinth.value,
				enableCurseForge: modlexEnableCurseForge.value,
				hideMusicTab: modlexHideMusicTab.value,
				hideMultiLaunch: modlexHideMultiLaunch.value,
			},
		}),
	)
}

watch(modlexHideServers, (v) => {
	localStorage.setItem(STORAGE_KEYS.hideServers, String(v))
	broadcast()
})
watch(modlexHideMusicTab, (v) => {
	localStorage.setItem(STORAGE_KEYS.hideMusicTab, String(v))
	broadcast()
})
watch(modlexHideMultiLaunch, (v) => {
	localStorage.setItem(STORAGE_KEYS.hideMultiLaunch, String(v))
	broadcast()
})
watch(modlexNewsSource, (v) => {
	localStorage.setItem(STORAGE_KEYS.newsSource, v)
	broadcast()
})
watch(modlexEnableModrinth, (v) => {
	localStorage.setItem(STORAGE_KEYS.enableModrinth, String(v))
	broadcast()
})
watch(modlexEnableCurseForge, (v) => {
	localStorage.setItem(STORAGE_KEYS.enableCurseForge, String(v))
	broadcast()
})
