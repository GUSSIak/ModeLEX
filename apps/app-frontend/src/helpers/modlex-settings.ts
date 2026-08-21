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
	consoleText: 'modlex_console_text',
	consoleScale: 'modlex_console_scale',
	consoleLetterGap: 'modlex_console_letter_gap',
	accentColor: 'modlex_accent_color',
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

function readNumber(key: string, fallback: number): number {
	const raw = localStorage.getItem(key)
	if (raw === null) return fallback
	const parsed = Number(raw)
	return Number.isFinite(parsed) ? parsed : fallback
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

// ── Консоль запуска (текст/размер/зазор пустого экрана) ──────────────────────
// Пустая строка = стандартный "NO SIGNAL". scale = 0 = автоподбор под размер
// терминала (см. BaseTerminal.vue::computeLetterScale).
export const modlexConsoleText = ref(readString(STORAGE_KEYS.consoleText, ''))
export const modlexConsoleScale = ref(readNumber(STORAGE_KEYS.consoleScale, 0))
export const modlexConsoleLetterGap = ref(readNumber(STORAGE_KEYS.consoleLetterGap, 2))

// ── Акцентный цвет ─────────────────────────────────────────────────────────
// Пустая строка = стандартный цвет темы (оверрайд не применяется).
export const modlexAccentColor = ref(readString(STORAGE_KEYS.accentColor, ''))

function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
	const match = /^#?([0-9a-f]{6})$/i.exec(hex.trim())
	if (!match) return null
	const num = parseInt(match[1], 16)
	return { r: (num >> 16) & 255, g: (num >> 8) & 255, b: num & 255 }
}

// Переменные, которые просто наследуют --color-brand через var(...) в
// variables.scss, оверрайдить отдельно не нужно — родительского --color-brand
// достаточно. Эти — с собственными rgba(...)-значениями, поэтому оверрайдим
// их тоже, тем же цветом на разной прозрачности.
const ALPHA_OVERRIDES: Record<string, number> = {
	'--color-brand-highlight': 0.25,
	'--color-brand-shadow': 0.7,
	'--brand-gradient-button': 0.1,
	'--brand-gradient-border': 0.2,
}

// Двухстоповые градиенты-подложки (карточки в новостях и т.п.) — пересобираем
// с тем же углом/раскладкой стопов, что и в оригинале, но обоими стопами от
// выбранного акцента. Не трогаем --brand-gradient-fade-out-color — это фейд
// именно в фон темы, а не акцентный градиент, перекраска его в акцент даст
// цветную дымку вместо плавного слияния с фоном.
const GRADIENT_VARS = ['--brand-gradient-bg', '--brand-gradient-strong-bg'] as const

/**
 * Оверрайдит --color-brand и производные инлайн-стилем на <html> — это бьёт
 * по специфичности любые правила из variables.scss, независимо от активной
 * темы.
 */
export function applyAccentColor(hex: string): void {
	const root = document.documentElement
	const rgb = hex ? hexToRgb(hex) : null
	if (!rgb) {
		root.style.removeProperty('--color-brand')
		for (const varName of Object.keys(ALPHA_OVERRIDES)) root.style.removeProperty(varName)
		for (const varName of GRADIENT_VARS) root.style.removeProperty(varName)
		return
	}
	root.style.setProperty('--color-brand', `#${hex.replace('#', '')}`)
	for (const [varName, alpha] of Object.entries(ALPHA_OVERRIDES)) {
		root.style.setProperty(varName, `rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, ${alpha})`)
	}
	const gradient = `linear-gradient(0deg, rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.22) 0%, rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.08) 100%)`
	for (const varName of GRADIENT_VARS) {
		root.style.setProperty(varName, gradient)
	}
}

applyAccentColor(modlexAccentColor.value)

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
watch(modlexConsoleText, (v) => {
	localStorage.setItem(STORAGE_KEYS.consoleText, v)
	broadcast()
})
watch(modlexConsoleScale, (v) => {
	localStorage.setItem(STORAGE_KEYS.consoleScale, String(v))
	broadcast()
})
watch(modlexConsoleLetterGap, (v) => {
	localStorage.setItem(STORAGE_KEYS.consoleLetterGap, String(v))
	broadcast()
})
watch(modlexAccentColor, (v) => {
	localStorage.setItem(STORAGE_KEYS.accentColor, v)
	applyAccentColor(v)
	broadcast()
})
