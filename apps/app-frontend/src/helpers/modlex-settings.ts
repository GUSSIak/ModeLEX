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
	hideFriends: 'modlex_hide_friends',
	hideRightSidebar: 'modlex_hide_right_sidebar',
	hideFloatingAccountWidget: 'modlex_hide_floating_account_widget',
	floatingGlassEffect: 'modlex_floating_glass_effect',
	consoleText: 'modlex_console_text',
	consoleScale: 'modlex_console_scale',
	consoleLetterGap: 'modlex_console_letter_gap',
	consoleFillChar: 'modlex_console_fill_char',
	consoleRainChars: 'modlex_console_rain_chars',
	consoleRainEnabled: 'modlex_console_rain_enabled',
	consoleFillColor: 'modlex_console_fill_color',
	consoleRainColor: 'modlex_console_rain_color',
	consoleBgColor: 'modlex_console_bg_color',
	accentColor: 'modlex_accent_color',
	bgColor: 'modlex_bg_color',
	panelColor: 'modlex_panel_color',
	textColor: 'modlex_text_color',
	iconColor: 'modlex_icon_color',
	dividerColor: 'modlex_divider_color',
	doubleBorderEnabled: 'modlex_double_border_enabled',
	doubleBorderInner: 'modlex_double_border_inner_color',
	doubleBorderOuter: 'modlex_double_border_outer_color',
	textOutlineEnabled: 'modlex_text_outline_enabled',
	textOutlineColor: 'modlex_text_outline_color',
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

// ── Правая панель ──────────────────────────────────────────────────────────
// Скрыть только блок "Друзья" внутри правой панели (панель при этом остаётся).
export const modlexHideFriends = ref(readBool(STORAGE_KEYS.hideFriends, false))
// Скрыть правую панель целиком (аккаунт/друзья/новости). Вместо неё — плавающая
// плашка текущего аккаунта (см. FloatingAccountWidget.vue), которую тоже можно
// скрыть по наведению — см. modlexHideFloatingAccountWidget ниже.
export const modlexHideRightSidebar = ref(readBool(STORAGE_KEYS.hideRightSidebar, false))
// Действует только при modlexHideRightSidebar — прячет и саму плавающую плашку
// за правый край экрана, показывая её только при наведении на угол.
export const modlexHideFloatingAccountWidget = ref(
	readBool(STORAGE_KEYS.hideFloatingAccountWidget, false),
)
// Полупрозрачный фон + блюр вместо сплошного — для плавающей плашки аккаунта
// и "подглядывающей" правой панели на Discover.
export const modlexFloatingGlassEffect = ref(readBool(STORAGE_KEYS.floatingGlassEffect, true))

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
export const modlexConsoleFillChar = ref(readString(STORAGE_KEYS.consoleFillChar, ''))
export const modlexConsoleRainChars = ref(readString(STORAGE_KEYS.consoleRainChars, ''))
// Отключает анимацию матричного дождя на пустом экране консоли — вместо неё
// статичный ASCII-арт спящего волка (см. BaseTerminal.vue::writeSleepingWolf).
export const modlexConsoleRainEnabled = ref(readBool(STORAGE_KEYS.consoleRainEnabled, false))
// Пустая строка = стандартный цвет (тёмно-серый для заполнителя, зелёный для дождя).
export const modlexConsoleFillColor = ref(readString(STORAGE_KEYS.consoleFillColor, ''))
export const modlexConsoleRainColor = ref(readString(STORAGE_KEYS.consoleRainColor, ''))
// Фон самой консоли — независимо от общего фона лаунчера (--surface-2 по умолчанию).
export const modlexConsoleBgColor = ref(readString(STORAGE_KEYS.consoleBgColor, ''))

/** Сбрасывает все настройки консоли (текст/размер/зазор/символы/цвета/дождь)
 * к значениям по умолчанию. */
export function resetConsoleSettings(): void {
	modlexConsoleText.value = ''
	modlexConsoleScale.value = 0
	modlexConsoleLetterGap.value = 2
	modlexConsoleFillChar.value = ''
	modlexConsoleRainChars.value = ''
	modlexConsoleRainEnabled.value = false
	modlexConsoleFillColor.value = ''
	modlexConsoleRainColor.value = ''
	modlexConsoleBgColor.value = ''
}

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

// Двухстоповые градиенты-подложки (карточки в новостях, "туман" снизу
// правой панели и т.п.) — пересобираем с тем же углом, но обоими стопами от
// выбранного акцента, чтобы не было рассинхрона с остальным акцентным UI.
const GRADIENT_VARS = [
	'--brand-gradient-bg',
	'--brand-gradient-strong-bg',
	'--brand-gradient-fade-out-color',
] as const

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
	const bgGradient = `linear-gradient(0deg, rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.22) 0%, rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.08) 100%)`
	root.style.setProperty('--brand-gradient-bg', bgGradient)
	root.style.setProperty('--brand-gradient-strong-bg', bgGradient)
	// "Туман" снизу карточек — фейд в ту же сторону, что в оригинале (сверху
	// вниз, от прозрачного к более плотному), просто тонирован акцентом.
	root.style.setProperty(
		'--brand-gradient-fade-out-color',
		`linear-gradient(to bottom, rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0) 0%, rgba(${rgb.r}, ${rgb.g}, ${rgb.b}, 0.35) 80%)`,
	)
}

applyAccentColor(modlexAccentColor.value)

// ── Расширенная кастомизация (фон, текст, иконки, разделители) ───────────────
// Пустая строка = стандартный цвет темы (оверрайд не применяется).
export const modlexBgColor = ref(readString(STORAGE_KEYS.bgColor, ''))
export const modlexPanelColor = ref(readString(STORAGE_KEYS.panelColor, ''))
export const modlexTextColor = ref(readString(STORAGE_KEYS.textColor, ''))
export const modlexIconColor = ref(readString(STORAGE_KEYS.iconColor, ''))
export const modlexDividerColor = ref(readString(STORAGE_KEYS.dividerColor, ''))
export const modlexDoubleBorderEnabled = ref(readBool(STORAGE_KEYS.doubleBorderEnabled, false))
export const modlexDoubleBorderInner = ref(readString(STORAGE_KEYS.doubleBorderInner, '#ffffff'))
export const modlexDoubleBorderOuter = ref(readString(STORAGE_KEYS.doubleBorderOuter, '#000000'))
export const modlexTextOutlineEnabled = ref(readBool(STORAGE_KEYS.textOutlineEnabled, false))
export const modlexTextOutlineColor = ref(readString(STORAGE_KEYS.textOutlineColor, '#000000'))

// Затемняет rgb на factor (0..1) в сторону чёрного — используется, чтобы из
// одного выбранного цвета текста получить иерархию "яркий/обычный/приглушённый",
// как в стандартной теме (--color-text-primary > -default > -tertiary).
function shade(rgb: { r: number; g: number; b: number }, factor: number): string {
	return `rgb(${Math.round(rgb.r * factor)}, ${Math.round(rgb.g * factor)}, ${Math.round(rgb.b * factor)})`
}

// Насколько каждый уровень surface-* светлее "базового" уровня своего яруса
// в стандартной тёмной теме (посчитано из variables.scss) — сохраняем эти же
// смещения поверх выбранного пользователем цвета, чтобы соседние уровни
// (например surface-4 относительно surface-3) остались читаемо светлее.
// Ярус "фон": сам контент (surface-1/1-5/2/2-5) — задаётся полем "Фон".
const BG_LEVEL_DELTAS: Record<string, { r: number; g: number; b: number }> = {
	'--surface-1': { r: 0, g: 0, b: 0 },
	'--surface-1-5': { r: 4, g: 4, b: 4 },
	'--surface-2': { r: 7, g: 7, b: 7 },
	'--surface-2-5': { r: 12, g: 12, b: 13 },
}
// Ярус "панели/карточки": приподнятые поверхности (surface-3/4/5) — кнопки,
// карточки инстансов, боковые панели. Задаётся отдельным полем "Панели и
// карточки", независимо от основного фона.
const PANEL_LEVEL_DELTAS: Record<string, { r: number; g: number; b: number }> = {
	'--surface-3': { r: 0, g: 0, b: 0 },
	'--surface-4': { r: 13, g: 13, b: 14 },
	'--surface-5': { r: 27, g: 27, b: 28 },
}

function clampChannel(v: number): number {
	return Math.max(0, Math.min(255, Math.round(v)))
}

function applySurfaceTier(
	hex: string,
	deltas: Record<string, { r: number; g: number; b: number }>,
): void {
	const root = document.documentElement
	const rgb = hex ? hexToRgb(hex) : null
	if (!rgb) {
		for (const varName of Object.keys(deltas)) root.style.removeProperty(varName)
		return
	}
	for (const [varName, delta] of Object.entries(deltas)) {
		const r = clampChannel(rgb.r + delta.r)
		const g = clampChannel(rgb.g + delta.g)
		const b = clampChannel(rgb.b + delta.b)
		root.style.setProperty(varName, `rgb(${r}, ${g}, ${b})`)
	}
}

/** Оверрайдит основной фон контента (surface-1/1-5/2/2-5). Панели и карточки
 * (surface-3/4/5) — отдельная настройка, см. applyPanelColor. */
export function applyBgColor(hex: string): void {
	applySurfaceTier(hex, BG_LEVEL_DELTAS)
}

/** Оверрайдит фон приподнятых поверхностей — карточки инстансов, кнопки,
 * боковые панели (surface-3/4/5). Не трогает основной фон, см. applyBgColor. */
export function applyPanelColor(hex: string): void {
	applySurfaceTier(hex, PANEL_LEVEL_DELTAS)
}

/** Оверрайдит цвет текста (contrast/default/tertiary) одним выбранным
 * цветом с производными более тёмными оттенками для сохранения иерархии. */
export function applyTextColor(hex: string): void {
	const root = document.documentElement
	const rgb = hex ? hexToRgb(hex) : null
	if (!rgb) {
		root.style.removeProperty('--color-text-primary')
		root.style.removeProperty('--color-text-default')
		root.style.removeProperty('--color-text-tertiary')
		return
	}
	root.style.setProperty('--color-text-primary', `#${hex.replace('#', '')}`)
	root.style.setProperty('--color-text-default', shade(rgb, 0.75))
	root.style.setProperty('--color-text-tertiary', shade(rgb, 0.6))
}

/** Оверрайдит цвет неактивных иконок бокового меню (--modlex-icon-color,
 * читается в NavButton.vue с фоллбэком на стандартный --color-text-default). */
export function applyIconColor(hex: string): void {
	const root = document.documentElement
	if (!hex) {
		root.style.removeProperty('--modlex-icon-color')
		return
	}
	root.style.setProperty('--modlex-icon-color', `#${hex.replace('#', '')}`)
}

/** Оверрайдит цвет разделителей (--color-divider). Цвет краёв карточек
 * (border-surface-4/5) теперь часть общего фона — см. applyBgColor. */
export function applyDividerColor(hex: string): void {
	const root = document.documentElement
	if (!hex) {
		root.style.removeProperty('--color-divider')
		return
	}
	root.style.setProperty('--color-divider', `#${hex.replace('#', '')}`)
}

/** Двойная обводка (внутренняя + внешняя) поверх стандартных краёв/границ —
 * см. глобальное CSS-правило [data-modlex-double-border] в App.vue. */
export function applyDoubleBorder(enabled: boolean, innerHex: string, outerHex: string): void {
	const root = document.documentElement
	if (enabled) {
		root.setAttribute('data-modlex-double-border', '')
		root.style.setProperty('--modlex-border-inner-color', innerHex || '#ffffff')
		root.style.setProperty('--modlex-border-outer-color', outerHex || '#000000')
	} else {
		root.removeAttribute('data-modlex-double-border')
		root.style.removeProperty('--modlex-border-inner-color')
		root.style.removeProperty('--modlex-border-outer-color')
	}
}

/** Обводка текста (--modlex-text-outline-color + [data-modlex-text-outline])
 * — независимо от двойной обводки карточек, см. глобальное CSS-правило
 * [data-modlex-text-outline] в App.vue. */
export function applyTextOutline(enabled: boolean, color: string): void {
	const root = document.documentElement
	if (enabled) {
		root.setAttribute('data-modlex-text-outline', '')
		root.style.setProperty('--modlex-text-outline-color', color || '#000000')
	} else {
		root.removeAttribute('data-modlex-text-outline')
		root.style.removeProperty('--modlex-text-outline-color')
	}
}

applyBgColor(modlexBgColor.value)
applyPanelColor(modlexPanelColor.value)
applyTextColor(modlexTextColor.value)
applyIconColor(modlexIconColor.value)
applyDividerColor(modlexDividerColor.value)
applyDoubleBorder(
	modlexDoubleBorderEnabled.value,
	modlexDoubleBorderInner.value,
	modlexDoubleBorderOuter.value,
)
applyTextOutline(modlexTextOutlineEnabled.value, modlexTextOutlineColor.value)

// ── Экспорт/импорт темы ───────────────────────────────────────────────────
interface ModlexThemeCode {
	v: 1
	accentColor: string
	bgColor: string
	panelColor: string
	textColor: string
	iconColor: string
	dividerColor: string
	doubleBorderEnabled: boolean
	doubleBorderInner: string
	doubleBorderOuter: string
	textOutlineEnabled: boolean
	textOutlineColor: string
}

/** Собирает все настройки кастомизации цвета в один code-строку (base64 от
 * JSON) — чтобы можно было сохранить/переслать тему одной строкой. */
export function exportThemeCode(): string {
	const data: ModlexThemeCode = {
		v: 1,
		accentColor: modlexAccentColor.value,
		bgColor: modlexBgColor.value,
		panelColor: modlexPanelColor.value,
		textColor: modlexTextColor.value,
		iconColor: modlexIconColor.value,
		dividerColor: modlexDividerColor.value,
		doubleBorderEnabled: modlexDoubleBorderEnabled.value,
		doubleBorderInner: modlexDoubleBorderInner.value,
		doubleBorderOuter: modlexDoubleBorderOuter.value,
		textOutlineEnabled: modlexTextOutlineEnabled.value,
		textOutlineColor: modlexTextOutlineColor.value,
	}
	return btoa(JSON.stringify(data))
}

/** Обратная операция — применяет код темы к текущим настройкам. Возвращает
 * false, если код нечитаем (неверный формат/повреждён). */
export function importThemeCode(code: string): boolean {
	let data: Partial<ModlexThemeCode>
	try {
		data = JSON.parse(atob(code.trim()))
	} catch {
		return false
	}
	if (typeof data !== 'object' || data === null) return false
	if (typeof data.accentColor === 'string') modlexAccentColor.value = data.accentColor
	if (typeof data.bgColor === 'string') modlexBgColor.value = data.bgColor
	if (typeof data.panelColor === 'string') modlexPanelColor.value = data.panelColor
	if (typeof data.textColor === 'string') modlexTextColor.value = data.textColor
	if (typeof data.iconColor === 'string') modlexIconColor.value = data.iconColor
	if (typeof data.dividerColor === 'string') modlexDividerColor.value = data.dividerColor
	if (typeof data.doubleBorderEnabled === 'boolean')
		modlexDoubleBorderEnabled.value = data.doubleBorderEnabled
	if (typeof data.doubleBorderInner === 'string')
		modlexDoubleBorderInner.value = data.doubleBorderInner
	if (typeof data.doubleBorderOuter === 'string')
		modlexDoubleBorderOuter.value = data.doubleBorderOuter
	if (typeof data.textOutlineEnabled === 'boolean')
		modlexTextOutlineEnabled.value = data.textOutlineEnabled
	if (typeof data.textOutlineColor === 'string')
		modlexTextOutlineColor.value = data.textOutlineColor
	return true
}

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
watch(modlexHideFriends, (v) => {
	localStorage.setItem(STORAGE_KEYS.hideFriends, String(v))
	broadcast()
})
watch(modlexHideRightSidebar, (v) => {
	localStorage.setItem(STORAGE_KEYS.hideRightSidebar, String(v))
	broadcast()
})
watch(modlexHideFloatingAccountWidget, (v) => {
	localStorage.setItem(STORAGE_KEYS.hideFloatingAccountWidget, String(v))
	broadcast()
})
watch(modlexFloatingGlassEffect, (v) => {
	localStorage.setItem(STORAGE_KEYS.floatingGlassEffect, String(v))
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
watch(modlexConsoleFillChar, (v) => {
	localStorage.setItem(STORAGE_KEYS.consoleFillChar, v)
	broadcast()
})
watch(modlexConsoleRainChars, (v) => {
	localStorage.setItem(STORAGE_KEYS.consoleRainChars, v)
	broadcast()
})
watch(modlexConsoleRainEnabled, (v) => {
	localStorage.setItem(STORAGE_KEYS.consoleRainEnabled, String(v))
	broadcast()
})
watch(modlexConsoleFillColor, (v) => {
	localStorage.setItem(STORAGE_KEYS.consoleFillColor, v)
	broadcast()
})
watch(modlexConsoleRainColor, (v) => {
	localStorage.setItem(STORAGE_KEYS.consoleRainColor, v)
	broadcast()
})
watch(modlexConsoleBgColor, (v) => {
	localStorage.setItem(STORAGE_KEYS.consoleBgColor, v)
	broadcast()
})
watch(modlexAccentColor, (v) => {
	localStorage.setItem(STORAGE_KEYS.accentColor, v)
	applyAccentColor(v)
	broadcast()
})
watch(modlexBgColor, (v) => {
	localStorage.setItem(STORAGE_KEYS.bgColor, v)
	applyBgColor(v)
	broadcast()
})
watch(modlexPanelColor, (v) => {
	localStorage.setItem(STORAGE_KEYS.panelColor, v)
	applyPanelColor(v)
	broadcast()
})
watch(modlexTextColor, (v) => {
	localStorage.setItem(STORAGE_KEYS.textColor, v)
	applyTextColor(v)
	broadcast()
})
watch(modlexIconColor, (v) => {
	localStorage.setItem(STORAGE_KEYS.iconColor, v)
	applyIconColor(v)
	broadcast()
})
watch(modlexDividerColor, (v) => {
	localStorage.setItem(STORAGE_KEYS.dividerColor, v)
	applyDividerColor(v)
	broadcast()
})
watch(
	[modlexDoubleBorderEnabled, modlexDoubleBorderInner, modlexDoubleBorderOuter],
	([enabled, inner, outer]) => {
		localStorage.setItem(STORAGE_KEYS.doubleBorderEnabled, String(enabled))
		localStorage.setItem(STORAGE_KEYS.doubleBorderInner, inner)
		localStorage.setItem(STORAGE_KEYS.doubleBorderOuter, outer)
		applyDoubleBorder(enabled, inner, outer)
		broadcast()
	},
)
watch([modlexTextOutlineEnabled, modlexTextOutlineColor], ([enabled, color]) => {
	localStorage.setItem(STORAGE_KEYS.textOutlineEnabled, String(enabled))
	localStorage.setItem(STORAGE_KEYS.textOutlineColor, color)
	applyTextOutline(enabled, color)
	broadcast()
})
