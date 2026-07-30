// apps/app-frontend/src/helpers/feature-flags.ts
//
// Локальный конфиг фич-флагов, который редактируется перед сборкой билда.
// Меняйте LOCAL_FEATURE_FLAGS ниже, чтобы отключить/заблокировать конкретную
// функцию в конкретной сборке — без изменений в остальном коде.
//
// Удалённый оверрайд (kill-switch с GitHub) живёт в modlex-feature-flags.ts —
// он подмешивается поверх этих значений во время работы приложения.

import { computed, ref } from 'vue'

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
export const LOCAL_FEATURE_FLAGS: FeatureFlags = {
	curseforge_platform: {
		enabled: false,
		locked: true,
		// message: 'свой текст, если нужно',
	},
}

export const featureFlags = ref<FeatureFlags>({ ...LOCAL_FEATURE_FLAGS })

export function useFeatureFlag(key: string) {
	const flag = computed(() => featureFlags.value[key])
	const locked = computed(() => flag.value?.locked === true)
	const enabled = computed(() => flag.value?.enabled !== false)
	const message = computed(() => flag.value?.message || DEFAULT_LOCKED_MESSAGE)
	return { flag, locked, enabled, message }
}
