// ModLEX: глобальный фон лаунчера (Home/Library) — shared-ref модуль, а не
// component-local state (см. workflow_shared_account_state в памяти сессии):
// и Settings-вкладка (пишет), и App.vue (рендерит слой фона) должны видеть
// одно и то же состояние сразу, без ручной синхронизации между разными
// смонтированными компонентами.
import { computed, ref } from 'vue'

import { get as getSettings, set as setSettings } from '@/helpers/settings'

export const globalBackgroundPath = ref<string | null>(null)
export const globalBackgroundOpacity = ref(0.5)
export const globalBackgroundBlurPx = ref(0)
export const globalBackgroundAnimated = ref(true)

const VIDEO_EXTENSIONS = new Set(['mp4', 'webm', 'mov', 'mkv'])

export const globalBackgroundIsVideo = computed(() => {
	const path = globalBackgroundPath.value
	if (!path) return false
	const ext = path.split('.').pop()?.toLowerCase() ?? ''
	return VIDEO_EXTENSIONS.has(ext)
})

export const globalBackgroundIsGif = computed(() => {
	const path = globalBackgroundPath.value
	if (!path) return false
	return (path.split('.').pop()?.toLowerCase() ?? '') === 'gif'
})

export const globalBackgroundIsAnimated = computed(
	() => globalBackgroundIsVideo.value || globalBackgroundIsGif.value,
)

export async function refreshGlobalBackground() {
	const settings = await getSettings()
	globalBackgroundPath.value = settings.modlex_global_background_path ?? null
	globalBackgroundOpacity.value = settings.modlex_global_background_opacity ?? 0.5
	globalBackgroundBlurPx.value = settings.modlex_global_background_blur_px ?? 0
	globalBackgroundAnimated.value = settings.modlex_global_background_animated ?? true
}

export async function persistGlobalBackground(patch: {
	path?: string | null
	opacity?: number
	blurPx?: number
	animated?: boolean
}) {
	if (patch.path !== undefined) globalBackgroundPath.value = patch.path
	if (patch.opacity !== undefined) globalBackgroundOpacity.value = patch.opacity
	if (patch.blurPx !== undefined) globalBackgroundBlurPx.value = patch.blurPx
	if (patch.animated !== undefined) globalBackgroundAnimated.value = patch.animated

	const settings = await getSettings()
	settings.modlex_global_background_path = globalBackgroundPath.value
	settings.modlex_global_background_opacity = globalBackgroundOpacity.value
	settings.modlex_global_background_blur_px = globalBackgroundBlurPx.value
	settings.modlex_global_background_animated = globalBackgroundAnimated.value
	await setSettings(settings)
}
