<template>
	<div class="flex flex-col gap-6">
		<Transition name="settings-notice-fade">
			<div v-if="inlineNotice" class="settings-notice sticky top-2 z-20 mx-auto">
				{{ inlineNotice }}
			</div>
		</Transition>

		<!-- Внешний вид -->
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

		<!-- Контент -->
		<div class="settings-section">
			<h2 class="settings-section__title">Контент</h2>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Источник новостей</h3>
					<p class="setting-row__desc">Откуда загружать новости в правой панели.</p>
				</div>
				<DropdownSelect
					v-model="modlexNewsSource"
					name="news-source"
					:options="newsSourceOptions"
					:get-option-label="getNewsLabel"
					class="settings-dropdown"
				/>
			</div>
		</div>

		<!-- Платформы поиска -->
		<div class="settings-section">
			<h2 class="settings-section__title">Платформы</h2>
			<p class="settings-section__desc">
				Управляйте источниками при поиске и установке модов. Хотя бы одна платформа должна
				оставаться включённой.
			</p>

			<!-- Modrinth -->
			<div class="setting-row platform-row">
				<div class="platform-logo modrinth-logo">
					<img src="https://cdn.modrinth.com/modrinth-new.png" alt="Modrinth" />
				</div>
				<div class="setting-row__info">
					<h3 class="setting-row__label">Modrinth</h3>
					<p class="setting-row__desc">Официальный магазин модов Modrinth.</p>
				</div>
				<Toggle :model-value="modlexEnableModrinth" @update:model-value="onToggleModrinth" />
			</div>

			<!-- CurseForge -->
			<div class="setting-row platform-row">
				<div class="platform-logo cf-logo">
					<svg viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
						<path d="M8 6h10l-3 7h5L9 28l3-11H7L8 6z" fill="#F16436" />
					</svg>
				</div>
				<div class="setting-row__info">
					<h3 class="setting-row__label">CurseForge</h3>
					<p class="setting-row__desc">Крупнейший архив модов для Minecraft.</p>
				</div>
				<div class="toggle-lock-wrapper" :class="{ 'toggle-lock-wrapper--locked': cfLocked }">
					<Toggle :model-value="cfDisplayValue" @update:model-value="onToggleCurseForge" />
				</div>
			</div>

			<p v-if="!modlexEnableModrinth && !cfDisplayValue" class="platform-warning">
				⚠ Включите хотя бы одну платформу.
			</p>
		</div>
	</div>
</template>

<script setup lang="ts">
import { DropdownSelect, Toggle } from '@modrinth/ui'
import { computed, onBeforeUnmount, ref } from 'vue'

import { useFeatureFlag } from '@/helpers/feature-flags'
import {
	modlexEnableCurseForge,
	modlexEnableModrinth,
	modlexHideServers,
	modlexNewsSource,
	type NewsSource,
} from '@/helpers/modlex-settings'

const newsSourceOptions: NewsSource[] = ['github', 'modrinth', 'off']

function getNewsLabel(value: NewsSource): string {
	return { github: 'GitHub', modrinth: 'Modrinth', off: 'Выключено' }[value] ?? value
}

const { locked: cfLocked, message: cfLockedMessage } = useFeatureFlag('curseforge_platform')

// CurseForge физически не работает, пока заблокирован фича-флагом — показываем
// тумблер выключенным, даже если пользователь когда-то включил его в настройках
// (это сохранённое значение вернётся, как только фича разблокируется).
const cfDisplayValue = computed(() => (cfLocked.value ? false : modlexEnableCurseForge.value))

const inlineNotice = ref<string | null>(null)
let inlineNoticeTimeout: ReturnType<typeof setTimeout> | null = null

function showInlineNotice(text: string) {
	if (inlineNoticeTimeout) clearTimeout(inlineNoticeTimeout)
	inlineNotice.value = text
	inlineNoticeTimeout = setTimeout(() => {
		inlineNotice.value = null
	}, 6000)
}

onBeforeUnmount(() => {
	if (inlineNoticeTimeout) clearTimeout(inlineNoticeTimeout)
})

function onToggleModrinth(value: boolean) {
	if (!value && modlexEnableModrinth.value && !cfDisplayValue.value) {
		showInlineNotice(
			'Нельзя отключить: должна остаться включённой хотя бы одна платформа, а CurseForge сейчас недоступен.',
		)
		return
	}
	modlexEnableModrinth.value = value
}

function onToggleCurseForge(value: boolean) {
	if (cfLocked.value) {
		showInlineNotice(cfLockedMessage.value)
		return
	}
	if (!value && modlexEnableCurseForge.value && !modlexEnableModrinth.value) {
		showInlineNotice('Нельзя отключить: должна остаться включённой хотя бы одна платформа.')
		return
	}
	modlexEnableCurseForge.value = value
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
	margin-bottom: 0.75rem;
	color: var(--color-contrast);
}

.settings-section__desc {
	margin: 0 0 1rem;
	font-size: 0.875rem;
	color: var(--color-secondary);
}

.setting-row {
	display: flex;
	align-items: center;
	gap: 1rem;
	padding: 0.5rem 0;
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

.platform-row {
	gap: 0.75rem;
}

.platform-logo {
	width: 2rem;
	height: 2rem;
	flex-shrink: 0;
	border-radius: 0.375rem;
	display: flex;
	align-items: center;
	justify-content: center;
	overflow: hidden;
}

.platform-logo img,
.platform-logo svg {
	width: 100%;
	height: 100%;
	object-fit: contain;
}

.modrinth-logo {
	background: #1bd96a1a;
}

.cf-logo {
	background: #f164361a;
}

.settings-dropdown {
	flex-shrink: 0;
	min-width: 140px;
}

.platform-warning {
	margin: 0.5rem 0 0;
	font-size: 0.8rem;
	color: var(--color-orange);
}

.toggle-lock-wrapper--locked {
	opacity: 0.5;
	filter: grayscale(1);
}

.toggle-lock-wrapper--locked :deep(button) {
	cursor: not-allowed;
}

.settings-notice {
	width: fit-content;
	max-width: 26rem;
	margin-bottom: 0.5rem;
	padding: 0.6rem 1.1rem;
	border-radius: 999px;
	border: 1px solid rgba(255, 255, 255, 0.25);
	background: rgba(59, 130, 246, 0.28);
	backdrop-filter: blur(16px);
	-webkit-backdrop-filter: blur(16px);
	box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
	color: #fff;
	font-size: 0.875rem;
	font-weight: 600;
	text-align: center;
}

.settings-notice-fade-enter-active,
.settings-notice-fade-leave-active {
	transition:
		opacity 0.2s ease,
		transform 0.2s ease;
}

.settings-notice-fade-enter-from,
.settings-notice-fade-leave-to {
	opacity: 0;
	transform: translateY(-0.5rem);
}
</style>
