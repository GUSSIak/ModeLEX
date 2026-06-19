// apps/app-frontend/src/helpers/modlex-github-news.ts
//
// Тянет новости из вашего собственного news.json (хостится на GitHub Pages
// или прямо через raw.githubusercontent.com — см. объяснение в чате),
// а не из GitHub Releases. Так в новости можно класть свой текст, картинки
// и ссылки в любом виде, без привязки к формату релизов.
//
// Требует (один раз, в конфиге Tauri):
// 1. apps/app/capabilities/plugins.json → в "http:default" → "allow" добавить
//    { "url": "https://YOUR_USERNAME.github.io/*" }
// 2. apps/app/tauri.conf.json → app.security.csp.connect-src добавить
//    https://YOUR_USERNAME.github.io
// (img-src в CSP уже разрешает любые https-картинки, так что превью новостей
// будут грузиться без дополнительных правок)

import { fetch as tauriFetch } from '@tauri-apps/plugin-http'

export interface ModlexNewsItem {
	id: string
	title: string
	date: string
	summary: string
	image?: string
	url?: string
	body?: string
}

// Полный путь до вашего news.json. Если используете GitHub Pages из репозитория
// modlex-site, это будет https://USERNAME.github.io/modlex-site/news.json
const NEWS_URL = 'https://gussiak.github.io/news.json'

const CACHE_KEY = 'modlex_news_cache'
const CACHE_TTL_MS = 10 * 60 * 1000 // 10 минут — не дёргаем сайт на каждое открытие лаунчера

interface CacheShape {
	fetchedAt: number
	items: ModlexNewsItem[]
}

export async function fetchModlexNews(): Promise<ModlexNewsItem[]> {
	const cached = readCache()
	if (cached && Date.now() - cached.fetchedAt < CACHE_TTL_MS) {
		return cached.items
	}

	try {
		const response = await tauriFetch(NEWS_URL, {
			headers: { Accept: 'application/json' },
		})

		if (!response.ok) {
			throw new Error(`news.json ответил статусом ${response.status}`)
		}

		const data = (await response.json()) as { items: ModlexNewsItem[] }
		const items = data.items ?? []

		writeCache({ fetchedAt: Date.now(), items })

		return items
	} catch (err) {
		// Сайт недоступен (например, нет интернета) — лучше показать
		// старый кэш, чем пустую ленту или ошибку
		console.warn('Не удалось обновить новости ModLEX:', err)
		return cached?.items ?? []
	}
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
