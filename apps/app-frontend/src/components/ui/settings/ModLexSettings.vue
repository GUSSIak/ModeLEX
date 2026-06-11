<script setup lang="ts">
	import { Toggle } from '@modrinth/ui'
	import { ref, watch } from 'vue'
	import { get, set } from '@/helpers/settings.ts'

	const settings = ref(await get())

	// Инициализируем поля в локальном объекте
	if (settings.value.modlex_hide_servers === undefined) {
	settings.value.modlex_hide_servers = false
	}
	if (settings.value.modlex_use_github_news === undefined) {
	settings.value.modlex_use_github_news = false
	}

	// Отладочная функция
	async function debugShowSettings() {
	const current = await get()
	console.log('=== DEBUG: ТЕКУЩИЕ НАСТРОЙКИ ===')
	console.log('modlex_hide_servers:', current.modlex_hide_servers)
	console.log('modlex_use_github_news:', current.modlex_use_github_news)
	alert(`modlex_hide_servers = ${current.modlex_hide_servers}\nmodlex_use_github_news = ${current.modlex_use_github_news}`)
	}

	watch(
	settings,
	async () => {
	console.log('=== WATCH СРАБОТАЛ ===')
	console.log('Сохраняем значения:', {
	modlex_hide_servers: settings.value.modlex_hide_servers,
	modlex_use_github_news: settings.value.modlex_use_github_news
	})

	// Получаем текущие глобальные настройки
	const currentSettings = await get()

	// Создаём новый объект с добавленными полями
	const newSettings = {
	...currentSettings,
	modlex_hide_servers: settings.value.modlex_hide_servers,
	modlex_use_github_news: settings.value.modlex_use_github_news
	}

	// Сохраняем
	await set(newSettings)

	// Отправляем событие
	window.dispatchEvent(new CustomEvent('modlex-settings-changed', {
	detail: {
	hideServers: settings.value.modlex_hide_servers,
	useGitHubNews: settings.value.modlex_use_github_news
	}
	}))
	},
	{ deep: true },
	)
</script>

<template>
	<div class="flex flex-col gap-4">
		<!-- Секция 1 -->
		<div class="settings-section">
			<h2 class="settings-section__title">Внешний вид</h2>
			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-base font-extrabold text-contrast">
						Скрыть вкладку "Серверы"
					</h3>
					<p class="m-0 text-sm text-secondary">
						Убирает кнопку "Серверы" из бокового меню лаунчера.
					</p>
				</div>
				<Toggle v-model="settings.modlex_hide_servers" />
			</div>
		</div>

		<!-- Секция 2 -->
		<div class="settings-section">
			<h2 class="settings-section__title">Контент</h2>
			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-base font-extrabold text-contrast">
						Использовать GitHub новости
					</h3>
					<p class="m-0 text-sm text-secondary">
						Показывать новости из GitHub вместо стандартной ленты Modrinth.
					</p>
				</div>
				<Toggle v-model="settings.modlex_use_github_news" />
			</div>
		</div>

		<!-- Отладочная кнопка -->
		<button @click="debugShowSettings"
				class="mt-4 p-2 bg-brand text-white rounded">
			🔍 Показать текущие настройки
		</button>
	</div>
</template>

<style scoped>
	.settings-section {
		padding-bottom: 1rem;
		margin-bottom: 1rem;
		border-bottom: 1px solid var(--color-divider);
	}

		.settings-section:last-child {
			border-bottom: none;
			margin-bottom: 0;
			padding-bottom: 0;
		}

	.settings-section__title {
		font-size: 1.1rem;
		font-weight: 600;
		margin-bottom: 1rem;
		color: var(--color-contrast);
	}
</style>
