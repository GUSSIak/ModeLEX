// apps/app-frontend/src/helpers/feature-flags.ts
//
// Локальный конфиг фич-флагов, который редактируется перед сборкой билда.
// Меняйте LOCAL_FEATURE_FLAGS ниже, чтобы отключить/заблокировать конкретную
// функцию в конкретной сборке — без изменений в остальном коде.
//
// Удалённый оверрайд (kill-switch с GitHub) живёт в modlex-feature-flags.ts —
// он подмешивается поверх этих значений во время работы приложения.

import { computed, ref } from 'vue'

import { useAppSettings } from '@/composables/use-app-settings.ts'

export interface FeatureFlag {
	/** Доступна ли функция пользователю */
	enabled: boolean
	/** Если true — пользователь не может переключить настройку сам, независимо от enabled */
	locked: boolean
	/** Свой текст баннера при попытке переключить заблокированную настройку */
	message?: string
}

export type FeatureFlags = Record<string, FeatureFlag>

export const DEFAULT_LOCKED_MESSAGE = 'Эта функция временно отключена разработчиком.'

// ===== РЕДАКТИРУЙТЕ ЗДЕСЬ ПЕРЕД СБОРКОЙ =====
// Незаконченные/непубличные фичи для открытого бета-теста: enabled: false (и/или
// locked: true) здесь прячет их для всех, но appSettings.devMode (секретная фраза,
// см. App.vue) раскрывает их обратно — см. enabled/locked ниже в useFeatureFlag.
export const LOCAL_FEATURE_FLAGS: FeatureFlags = {
	curseforge_platform: {
		enabled: false,
		locked: true,
		// message: 'свой текст, если нужно',
	},
	multi_account_launch: {
		enabled: false,
	},
	modlex_music: {
		enabled: false,
	},
	// Гейт с кодом тестировщика для бета-сборок — управляется удалённо через
	// flags.json (см. modlex-feature-flags.ts). Локальный дефолт ВЫКЛЮЧЕН:
	// этот флаг не задан ни в одном локальном билде здесь, поэтому если
	// flags.json недоступен/битый (нет сети, GitHub Pages лежит, опечатка в
	// JSON и т.п.), гейт не должен внезапно требовать код у всех — лучше
	// молча пропустить, чем заблокировать приложение из-за сетевого сбоя.
	beta_tester_gate: {
		enabled: false,
	},
	// Отдельные замки на переключение канала обновлений (Настройки → ModLEX →
	// Обновления) — например, чтобы удержать текущих тестировщиков на beta
	// (locked: true на switch_to_stable_channel) или наоборот прекратить
	// набор новых (locked: true на switch_to_beta_channel), не трогая уже
	// одобренных. По умолчанию оба разрешены — как было раньше.
	switch_to_stable_channel: {
		enabled: true,
	},
	switch_to_beta_channel: {
		enabled: true,
	},
}

export const featureFlags = ref<FeatureFlags>({ ...LOCAL_FEATURE_FLAGS })

export function useFeatureFlag(key: string) {
	const appSettings = useAppSettings()
	const flag = computed(() => featureFlags.value[key])
	const locked = computed(() => !appSettings.devMode && flag.value?.locked === true)
	const enabled = computed(() => appSettings.devMode || flag.value?.enabled !== false)
	const message = computed(() => flag.value?.message || DEFAULT_LOCKED_MESSAGE)
	return { flag, locked, enabled, message }
}
