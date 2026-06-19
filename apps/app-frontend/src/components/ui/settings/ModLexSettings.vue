<script setup lang="ts">
	import { Toggle } from '@modrinth/ui'
	import { modlexHideServers, modlexUseGithubNews, modlexHideNews } from '@/helpers/modlex-settings'
</script>

<template>
	<div class="flex flex-col gap-4">
		<!-- Секция 1: Внешний вид -->
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
				<Toggle v-model="modlexHideServers" />
			</div>
		</div>

		<!-- Секция 2: Контент / новости -->
		<div class="settings-section">
			<h2 class="settings-section__title">Контент</h2>

			<div class="flex items-center justify-between gap-4">
				<div>
					<h3 class="m-0 text-base font-extrabold text-contrast">
						Использовать GitHub новости
					</h3>
					<p class="m-0 text-sm text-secondary">
						Показывать новости из GitHub-репозитория ModLEX вместо стандартной ленты Modrinth.
					</p>
				</div>
				<Toggle v-model="modlexUseGithubNews" />
			</div>

			<div class="flex items-center justify-between gap-4 mt-4">
				<div>
					<h3 class="m-0 text-base font-extrabold text-contrast">
						Скрыть ленту новостей
					</h3>
					<p class="m-0 text-sm text-secondary">
						Полностью убирает блок новостей из правой части интерфейса.
					</p>
				</div>
				<Toggle v-model="modlexHideNews" />
			</div>
		</div>
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
