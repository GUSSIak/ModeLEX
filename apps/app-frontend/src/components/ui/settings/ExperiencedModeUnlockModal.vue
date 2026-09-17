<template>
	<NewModal ref="stepOneModal" header="Настройки для опытных пользователей" max-width="32rem">
		<div class="flex flex-col gap-4">
			<p class="m-0 text-sm text-secondary">
				Здесь собраны настройки, которые либо рискованны для обычного пользователя, либо требуют
				понимания последствий (например, временная правка системного файла hosts). Используйте их,
				только если понимаете, что делаете, какие могут быть последствия и проблемы, и заранее
				изучили, что делает каждая конкретная настройка. Мы не несём ответственности за проблемы,
				возникшие из-за их использования.
			</p>
			<div class="flex gap-2 justify-end items-stretch">
				<Button
					class="flex-1 h-auto py-2"
					style="white-space: normal; text-align: left; line-height: 1.3"
					@click="cancel"
				>
					Я не разбираюсь и не уверен в своих действиях
				</Button>
				<Button type="colored" color="orange" class="shrink-0" @click="goToStepTwo">
					Я уверен
				</Button>
			</div>
		</div>
	</NewModal>

	<NewModal ref="stepTwoModal" header="Вы точно уверены?" max-width="26rem">
		<div class="flex flex-col gap-4">
			<p class="m-0 text-sm text-secondary">
				Последний шанс передумать — дальше будут открыты потенциально рискованные настройки.
			</p>
			<div class="flex gap-2 justify-end">
				<Button @click="cancel">Нет, я не опытный</Button>
				<Button type="colored" color="orange" @click="confirm">Да, я опытный</Button>
			</div>
		</div>
	</NewModal>

	<Teleport to="body">
		<Transition name="ok-splash-fade">
			<div v-if="showOkSplash" class="ok-splash-overlay">
				<div class="ok-splash-box">OK</div>
			</div>
		</Transition>
	</Teleport>
</template>

<script setup lang="ts">
import { Button, NewModal } from '@modrinth/ui'
import { ref } from 'vue'

import { modlexExperiencedModeUnlocked } from '@/helpers/modlex-settings'

const stepOneModal = ref<InstanceType<typeof NewModal>>()
const stepTwoModal = ref<InstanceType<typeof NewModal>>()
const showOkSplash = ref(false)

function show() {
	stepOneModal.value?.show()
}

function goToStepTwo() {
	stepOneModal.value?.hide()
	stepTwoModal.value?.show()
}

function cancel() {
	stepOneModal.value?.hide()
	stepTwoModal.value?.hide()
}

function confirm() {
	stepTwoModal.value?.hide()
	modlexExperiencedModeUnlocked.value = true
	showOkSplash.value = true
	setTimeout(() => {
		showOkSplash.value = false
	}, 1600)
}

defineExpose({ show })
</script>

<style scoped>
.ok-splash-overlay {
	position: fixed;
	inset: 0;
	z-index: 9999;
	display: flex;
	align-items: center;
	justify-content: center;
	pointer-events: none;
}

.ok-splash-box {
	background: var(--color-raised-bg);
	border: 1px solid var(--color-button-bg);
	border-radius: 1rem;
	padding: 1.5rem 3rem;
	font-size: 2rem;
	font-weight: 700;
	color: var(--color-contrast);
	box-shadow: 0 10px 40px rgba(0, 0, 0, 0.35);
}

.ok-splash-fade-enter-active {
	transition:
		opacity 0.15s ease,
		transform 0.15s ease;
}
.ok-splash-fade-leave-active {
	transition:
		opacity 0.5s ease,
		transform 0.5s ease;
}
.ok-splash-fade-enter-from,
.ok-splash-fade-leave-to {
	opacity: 0;
	transform: scale(0.9);
}
</style>
