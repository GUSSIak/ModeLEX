<template>
	<div class="flex flex-col gap-6">
		<!-- Секция: Внешний вид -->
		<div class="settings-section">
			<h2 class="settings-section__title">Внешний вид</h2>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Скрыть вкладку "Серверы"</h3>
					<p class="setting-row__desc">Убирает кнопку серверов из бокового меню.</p>
				</div>
				<Toggle v-model="modlexHideServers" />
			</div>
		</div>

		<!-- Секция: Контент -->
		<div class="settings-section">
			<h2 class="settings-section__title">Контент</h2>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Источник новостей</h3>
					<p class="setting-row__desc">Откуда загружать новости в правой панели.</p>
				</div>
				<DropdownSelect v-model="modlexNewsSource"
								name="news-source"
								:options="newsSourceOptions"
								:get-option-label="getNewsLabel"
								class="news-dropdown" />
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
	import { DropdownSelect, Toggle } from '@modrinth/ui'
	import { modlexHideServers, modlexNewsSource, type NewsSource } from '@/helpers/modlex-settings'

	const newsSourceOptions: NewsSource[] = ['github', 'modrinth', 'off']

	function getNewsLabel(value: NewsSource): string {
	const labels: Record<NewsSource, string> = {
	github: 'GitHub',
	modrinth: 'Modrinth',
	off: 'Выключено',
	}
	return labels[value] ?? value
	}
</script>

<style scoped>
	.settings-section {
		padding-bottom: 1.25rem;
		margin-bottom: 1.25rem;
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

	.setting-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1.5rem;
	}

	.setting-row__info {
		flex: 1;
		min-width: 0;
	}

	.setting-row__label {
		margin: 0 0 0.2rem;
		font-size: 1rem;
		font-weight: 700;
		color: var(--color-contrast);
	}

	.setting-row__desc {
		margin: 0;
		font-size: 0.875rem;
		color: var(--color-secondary);
	}

	.news-dropdown {
		flex-shrink: 0;
		min-width: 140px;
	}
</style>
