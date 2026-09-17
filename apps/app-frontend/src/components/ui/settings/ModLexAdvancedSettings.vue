<template>
	<div class="flex flex-col gap-6">
		<div class="settings-section">
			<h2 class="settings-section__title">Автофикс мультиплеера офлайн/Ely.by-аккаунтов</h2>
			<p class="m-0 text-sm text-secondary">
				На некоторых версиях (подтверждено на 1.16.5) ванильный клиент блокирует Multiplayer для
				офлайн- и Ely.by-аккаунтов, только если интернет доступен во время запуска. Этот тумблер
				на несколько секунд перенаправляет auth/session-хосты Mojang на localhost через файл hosts,
				затем возвращает как было.
			</p>
			<p class="m-0 text-sm text-secondary">
				<strong>Требует запуска ModLEX App с правами администратора</strong> — ограничение самого
				файла hosts, не Java. Не закрывайте ModLEX App принудительно во время запуска игры, пока
				фикс активен — при обычном крэше он подчистит себя сам при следующем старте, но лучше не
				проверять. Может временно повлиять на другие приложения, которые тоже стучатся в Mojang.
			</p>
			<p v-if="!canWriteHosts" class="m-0 text-sm text-orange">
				ModLEX App сейчас не может писать в файл hosts — перезапустите его от имени администратора,
				чтобы фикс реально применялся.
			</p>

			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Включить автофикс</h3>
				</div>
				<Toggle :model-value="fixEnabled" @update:model-value="onToggleAttempt" />
			</div>

			<div class="mt-2">
				<Button
					type="transparent"
					native-type="button"
					:disabled="restoring"
					@click="restoreHosts"
				>
					<SpinnerIcon v-if="restoring" class="animate-spin" />
					<HistoryIcon v-else />
					Восстановить hosts-файл
				</Button>
			</div>
		</div>

		<ConfirmModal
			ref="enableFixModal"
			title="Включить автофикс мультиплеера?"
			description="Потребуется перезапустить ModLEX App от имени администратора, чтобы это реально сработало. На несколько секунд при каждом запуске игры (с офлайн/Ely.by-аккаунтом) файл hosts будет временно изменён. Не закрывайте ModLEX App принудительно, пока идёт запуск."
			proceed-label="Включить"
			danger
			@proceed="confirmEnable"
		/>
	</div>
</template>

<script setup lang="ts">
import { HistoryIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, ConfirmModal, injectNotificationManager, Toggle } from '@modrinth/ui'
import { onMounted, ref } from 'vue'

import {
	get as getSettings,
	modlexCanWriteHostsFile,
	modlexRestoreHostsFile,
	set as setSettings,
} from '@/helpers/settings'

const { addNotification, handleError } = injectNotificationManager()

const enableFixModal = ref<InstanceType<typeof ConfirmModal>>()
const fixEnabled = ref(false)
const canWriteHosts = ref(true)
const restoring = ref(false)

onMounted(async () => {
	fixEnabled.value =
		(await getSettings()).modlex_experimental_offline_multiplayer_fix ?? false
	canWriteHosts.value = await modlexCanWriteHostsFile().catch(() => true)
})

function onToggleAttempt(value: boolean) {
	if (!value) {
		persist(false)
		return
	}
	enableFixModal.value?.show()
}

async function confirmEnable() {
	await persist(true)
}

async function persist(value: boolean) {
	fixEnabled.value = value
	const settings = await getSettings()
	settings.modlex_experimental_offline_multiplayer_fix = value
	await setSettings(settings)
}

async function restoreHosts() {
	restoring.value = true
	try {
		const restored = await modlexRestoreHostsFile()
		addNotification({
			type: restored ? 'success' : 'info',
			title: restored ? 'Hosts-файл восстановлен' : 'Нечего восстанавливать',
			text: restored
				? 'Файл hosts возвращён к исходному состоянию из резервной копии.'
				: 'Резервная копия ещё не создавалась — автофикс ни разу не запускался.',
		})
	} catch (error) {
		handleError(error)
	} finally {
		restoring.value = false
	}
}
</script>
