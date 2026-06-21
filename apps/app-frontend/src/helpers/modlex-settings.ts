// apps/app-frontend/src/helpers/modlex-settings.ts

import { ref, watch, computed } from 'vue'

const STORAGE_KEYS = {
	hideServers: 'modlex_hide_servers',
	newsSource: 'modlex_news_source',
} as const

export type NewsSource = 'github' | 'modrinth' | 'off'

function readString(key: string, fallback: string): string {
	const raw = localStorage.getItem(key)
	if (raw === null) return fallback
	return raw
}

function readBool(key: string, fallback: boolean): boolean {
	const raw = localStorage.getItem(key)
	if (raw === null) return fallback
	return raw === 'true'
}

export const modlexHideServers = ref(readBool(STORAGE_KEYS.hideServers, false))
export const modlexNewsSource = ref<NewsSource>(
	readString(STORAGE_KEYS.newsSource, 'github') as NewsSource,
)

// Computed для удобства использования в компонентах
export const modlexUseGithubNews = computed(() => modlexNewsSource.value === 'github')
export const modlexHideNews = computed(() => modlexNewsSource.value === 'off')
export const modlexUseModrinthNews = computed(() => modlexNewsSource.value === 'modrinth')

function broadcast() {
	window.dispatchEvent(
		new CustomEvent('modlex-settings-changed', {
			detail: {
				hideServers: modlexHideServers.value,
				newsSource: modlexNewsSource.value,
			},
		}),
	)
}

watch(modlexHideServers, (value) => {
	localStorage.setItem(STORAGE_KEYS.hideServers, String(value))
	broadcast()
})

watch(modlexNewsSource, (value) => {
	localStorage.setItem(STORAGE_KEYS.newsSource, value)
	broadcast()
})
