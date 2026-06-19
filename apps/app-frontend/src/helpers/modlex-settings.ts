// apps/app-frontend/src/helpers/modlex-settings.ts
//
// Почему не через settings.ts (get/set) как было сделано раньше:
// Rust-структура Settings (packages/app-lib/src/state/settings.rs) перечисляет
// строго фиксированный набор полей и пишет их в строго определённые колонки
// SQLite. Когда из Vue вызывается set({ ...currentSettings, modlex_hide_servers: true }),
// Tauri десериализует объект в эту Rust-структуру — все "лишние" поля,
// которых нет в структуре (modlex_hide_servers, modlex_use_github_news),
// молча отбрасываются. Settings::update() сохраняет в БД только те же
// фиксированные колонки. После перезапуска settings_get() вернёт объект
// БЕЗ этих полей — отсюда и сброс ползунка.
//
// Чтобы не трогать Rust/SQL (и не плодить конфликты при будущих апдейтах
// апстрима Modrinth), храним ModLEX-настройки отдельно, в localStorage.
// Это тот же подход, который уже используется в самом Modrinth App —
// см. PrideFundraiserBanner.vue и логику опросов в App.vue.

import { ref, watch } from 'vue'

const STORAGE_KEYS = {
	hideServers: 'modlex_hide_servers',
	useGithubNews: 'modlex_use_github_news',
	hideNews: 'modlex_hide_news',
} as const

function readBool(key: string, fallback: boolean): boolean {
	const raw = localStorage.getItem(key)
	if (raw === null) return fallback
	return raw === 'true'
}

// Реактивные синглтоны: любой компонент, импортирующий эти ref,
// получает ОДИН и тот же инстанс — значения остаются синхронными
// между настройками, навигацией и панелью новостей без лишней магии.
export const modlexHideServers = ref(readBool(STORAGE_KEYS.hideServers, false))
export const modlexUseGithubNews = ref(readBool(STORAGE_KEYS.useGithubNews, true)) // по умолчанию включено
export const modlexHideNews = ref(readBool(STORAGE_KEYS.hideNews, false))

function broadcast() {
	window.dispatchEvent(
		new CustomEvent('modlex-settings-changed', {
			detail: {
				hideServers: modlexHideServers.value,
				useGithubNews: modlexUseGithubNews.value,
				hideNews: modlexHideNews.value,
			},
		}),
	)
}

watch(modlexHideServers, (value) => {
	localStorage.setItem(STORAGE_KEYS.hideServers, String(value))
	broadcast()
})

watch(modlexUseGithubNews, (value) => {
	localStorage.setItem(STORAGE_KEYS.useGithubNews, String(value))
	broadcast()
})

watch(modlexHideNews, (value) => {
	localStorage.setItem(STORAGE_KEYS.hideNews, String(value))
	broadcast()
})
