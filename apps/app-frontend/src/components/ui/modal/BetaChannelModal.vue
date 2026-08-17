<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" max-width="420px">
		<div class="flex flex-col gap-3">
			<p class="m-0 text-sm text-secondary">{{ formatMessage(messages.description) }}</p>

			<div class="flex gap-2">
				<input
					v-model="code"
					type="text"
					class="w-full rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast outline-none focus:border-brand"
					:placeholder="formatMessage(messages.codePlaceholder)"
					:disabled="checking"
					@keydown.enter="submit"
				/>
			</div>

			<Admonition
				v-if="result?.status === 'invalid'"
				type="critical"
				:header="formatMessage(messages.invalidHeader)"
				:body="formatMessage(messages.invalidBody)"
			/>

			<Admonition
				v-if="result?.status === 'pending'"
				type="warning"
				:header="formatMessage(messages.pendingHeader)"
			>
				<p class="m-0 mb-2 text-sm">{{ formatMessage(messages.pendingBody) }}</p>
				<div class="flex items-center gap-2">
					<code class="flex-1 truncate rounded-lg bg-surface-1 px-2 py-1 text-xs">{{
						result.testerId
					}}</code>
					<ButtonStyled size="small">
						<button type="button" @click="copyTesterId">
							<CopyIcon />
							{{ formatMessage(commonMessages.copyButton) }}
						</button>
					</ButtonStyled>
				</div>
			</Admonition>

			<Admonition
				v-if="result?.status === 'approved'"
				type="success"
				:header="formatMessage(messages.approvedHeader)"
				:body="formatMessage(messages.approvedBody)"
			/>

			<div v-if="result?.status !== 'approved'" class="flex flex-col gap-2">
				<span class="text-xs text-secondary">{{ formatMessage(messages.getCodeHint) }}</span>
				<div class="flex gap-2">
					<ButtonStyled type="outlined" size="small">
						<a :href="TELEGRAM_URL" target="_blank" rel="noopener">
							<ExternalIcon />
							Telegram
						</a>
					</ButtonStyled>
					<ButtonStyled type="outlined" size="small">
						<a :href="DISCORD_URL" target="_blank" rel="noopener">
							<ExternalIcon />
							Discord
						</a>
					</ButtonStyled>
				</div>
			</div>
		</div>

		<template #actions>
			<div class="flex justify-end gap-2">
				<ButtonStyled type="outlined">
					<button type="button" @click="modal?.hide()">
						{{ formatMessage(commonMessages.cancelButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button type="button" :disabled="!code.trim() || checking" @click="submit">
						<SpinnerIcon v-if="checking" class="animate-spin" />
						{{ formatMessage(messages.submitButton) }}
					</button>
				</ButtonStyled>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import { CopyIcon, ExternalIcon, SpinnerIcon } from '@modrinth/assets'
import {
	Admonition,
	ButtonStyled,
	commonMessages,
	defineMessages,
	injectNotificationManager,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { ref } from 'vue'

import { checkTesterCode, type CodeCheckResult } from '@/helpers/modlex-beta-channel'

const TELEGRAM_URL = 'https://t.me/modlexapp'
const DISCORD_URL = 'https://discord.gg/Eqn2JXcmv7'

const { formatMessage } = useVIntl()
const { handleError, addNotification } = injectNotificationManager()

const emit = defineEmits<{
	approved: []
}>()

const messages = defineMessages({
	header: {
		id: 'app.settings.beta-channel.header',
		defaultMessage: 'Enable beta channel',
	},
	description: {
		id: 'app.settings.beta-channel.description',
		defaultMessage: 'Enter a tester code to switch this launcher to the beta update channel.',
	},
	codePlaceholder: {
		id: 'app.settings.beta-channel.code-placeholder',
		defaultMessage: 'Tester code',
	},
	submitButton: {
		id: 'app.settings.beta-channel.submit',
		defaultMessage: 'Check code',
	},
	invalidHeader: {
		id: 'app.settings.beta-channel.invalid-header',
		defaultMessage: 'Invalid code',
	},
	invalidBody: {
		id: 'app.settings.beta-channel.invalid-body',
		defaultMessage: 'This code was not recognized.',
	},
	pendingHeader: {
		id: 'app.settings.beta-channel.pending-header',
		defaultMessage: 'Code accepted — one more step',
	},
	pendingBody: {
		id: 'app.settings.beta-channel.pending-body',
		defaultMessage: 'Send this ID to whoever gave you the code so they can approve this device.',
	},
	approvedHeader: {
		id: 'app.settings.beta-channel.approved-header',
		defaultMessage: 'Approved',
	},
	approvedBody: {
		id: 'app.settings.beta-channel.approved-body',
		defaultMessage: 'Switching to the beta channel and downloading the latest beta build.',
	},
	getCodeHint: {
		id: 'app.settings.beta-channel.get-code-hint',
		defaultMessage: "Don't have a code?",
	},
})

const modal = ref<InstanceType<typeof NewModal>>()
const code = ref('')
const checking = ref(false)
const result = ref<CodeCheckResult | null>(null)

async function submit() {
	if (!code.value.trim() || checking.value) return
	checking.value = true
	try {
		result.value = await checkTesterCode(code.value)
		if (result.value.status === 'approved') {
			emit('approved')
		}
	} catch (e) {
		handleError(e)
	} finally {
		checking.value = false
	}
}

async function copyTesterId() {
	if (result.value?.status !== 'pending') return
	await navigator.clipboard.writeText(result.value.testerId)
	addNotification({
		title: formatMessage(commonMessages.copyButton),
		text: result.value.testerId,
		type: 'success',
	})
}

function show() {
	code.value = ''
	result.value = null
	modal.value?.show()
}

function hide() {
	modal.value?.hide()
}

defineExpose({
	show,
	hide,
})
</script>
