<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" max-width="420px">
		<Admonition
			v-if="showLowRamWarning"
			type="warning"
			class="mb-3"
			:header="formatMessage(messages.lowRamWarningHeader)"
			:body="formatMessage(messages.lowRamWarningBody)"
		/>

		<div class="flex flex-col gap-2">
			<div v-for="account in accounts" :key="account.id" class="flex flex-col gap-1.5">
				<div class="flex w-full items-center gap-2 rounded-xl bg-surface-3 p-2">
					<button
						type="button"
						class="flex min-w-0 flex-1 items-center gap-2 border-0 bg-transparent p-0 cursor-pointer button-base"
						@click="toggleSelected(account.id)"
					>
						<RadioButtonCheckedIcon
							v-if="selectedIds.has(account.id)"
							class="w-5 h-5 text-brand shrink-0"
						/>
						<RadioButtonIcon v-else class="w-5 h-5 text-secondary shrink-0" />
						<Avatar :src="avatarUrl(account)" size="24px" />
						<p class="m-0 min-w-0 flex-1 truncate text-left text-contrast">{{ account.name }}</p>
					</button>
					<OnlineIndicatorIcon
						v-if="runningAccountIds.has(account.id)"
						v-tooltip="formatMessage(messages.alreadyRunning)"
						class="shrink-0"
					/>
					<span
						v-if="account.kind === 'elyby'"
						v-tooltip="'Ely.by'"
						class="inline-flex size-4 shrink-0 items-center justify-center rounded-full text-[9px] font-bold text-white"
						style="background-color: #00b6a5"
						>E</span
					>
					<span
						v-else-if="account.kind === 'offline'"
						v-tooltip="formatMessage(messages.offlineAccount)"
						class="inline-flex size-4 shrink-0 items-center justify-center rounded-full bg-surface-5 text-secondary"
					>
						<WifiOffIcon class="w-3 h-3" />
					</span>
					<button
						v-if="advancedMode && selectedIds.has(account.id)"
						v-tooltip="formatMessage(messages.advancedOptionsToggle)"
						type="button"
						class="flex shrink-0 items-center justify-center rounded-lg border-0 bg-transparent p-1 cursor-pointer text-secondary hover:bg-surface-5 hover:text-contrast"
						:aria-label="formatMessage(messages.advancedOptionsToggle)"
						@click="toggleExpanded(account.id)"
					>
						<DropdownIcon
							class="w-4 h-4 transition-transform"
							:class="{ 'rotate-180': expandedIds.has(account.id) }"
						/>
					</button>
				</div>

				<div
					v-if="advancedMode && selectedIds.has(account.id) && expandedIds.has(account.id)"
					class="ml-8 flex flex-col gap-2 rounded-lg bg-surface-2 p-2"
				>
					<div class="flex items-center gap-2">
						<label class="text-xs text-secondary" :for="`memory-${account.id}`">
							{{ formatMessage(messages.memoryLabel) }}
						</label>
						<input
							:id="`memory-${account.id}`"
							v-model.number="overrides[account.id].memoryMb"
							type="number"
							min="512"
							step="512"
							class="w-24 rounded-lg border border-solid border-surface-5 bg-button-bg px-2 py-1 text-sm text-contrast outline-none focus:border-brand"
						/>
						<span class="text-xs text-secondary">{{ formatMessage(messages.memoryUnit) }}</span>
					</div>
					<input
						v-model="overrides[account.id].extraArgs"
						type="text"
						class="w-full rounded-lg border border-solid border-surface-5 bg-button-bg px-2 py-1 text-sm text-contrast outline-none focus:border-brand"
						:placeholder="formatMessage(messages.extraArgsPlaceholder)"
					/>
				</div>
			</div>
			<p v-if="accounts.length === 0" class="text-sm text-secondary m-0">
				{{ formatMessage(messages.noAccounts) }}
			</p>
		</div>

		<Checkbox
			v-model="advancedMode"
			class="mt-4"
			:label="formatMessage(messages.advancedModeHint)"
			label-class="text-sm text-secondary"
		/>

		<template #actions>
			<div class="flex justify-end">
				<Button
					type="colored"
					color="brand"
					native-type="button"
					:disabled="selectedIds.size === 0"
					@click="confirmLaunch"
				>
					<PlayIcon />
					{{ formatMessage(commonMessages.playButton) }}
				</Button>
			</div>
		</template>
	</NewModal>
</template>

<script setup lang="ts">
import {
	DropdownIcon,
	OnlineIndicatorIcon,
	PlayIcon,
	RadioButtonCheckedIcon,
	RadioButtonIcon,
	TagCategoryWifiOffIcon as WifiOffIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Avatar,
	Button,
	Checkbox,
	commonMessages,
	defineMessages,
	NewModal,
	useVIntl,
} from '@modrinth/ui'
import { computed, reactive, ref, watch } from 'vue'

export interface MultiLaunchAccount {
	id: string
	name: string
	kind: 'microsoft' | 'offline' | 'elyby'
}

export interface MultiLaunchSelection {
	accountId: string
	memoryMb?: number
	extraLaunchArgs?: string[]
}

const props = withDefaults(
	defineProps<{
		accounts: MultiLaunchAccount[]
		runningAccountIds?: Set<string>
		totalMemoryMb?: number | null
	}>(),
	{
		runningAccountIds: () => new Set(),
		totalMemoryMb: null,
	},
)

const emit = defineEmits<{
	launch: [selections: MultiLaunchSelection[]]
}>()

const { formatMessage } = useVIntl()

const messages = defineMessages({
	header: {
		id: 'app.instance.multi-launch.header',
		defaultMessage: 'Launch as multiple accounts',
	},
	noAccounts: {
		id: 'app.instance.multi-launch.no-accounts',
		defaultMessage: 'No accounts signed in.',
	},
	alreadyRunning: {
		id: 'app.instance.multi-launch.already-running',
		defaultMessage: 'Already running this instance',
	},
	lowRamWarningHeader: {
		id: 'app.instance.multi-launch.low-ram-warning-header',
		defaultMessage: 'Your device has limited memory',
	},
	lowRamWarningBody: {
		id: 'app.instance.multi-launch.low-ram-warning-body',
		defaultMessage:
			'Launching several accounts at once can be heavy on lower-memory devices. Launches will be spaced out more to reduce load, but consider launching fewer accounts at a time.',
	},
	offlineAccount: {
		id: 'app.instance.multi-launch.offline-account',
		defaultMessage: 'Offline account',
	},
	advancedModeHint: {
		id: 'app.instance.multi-launch.advanced-mode-hint',
		defaultMessage: 'Advanced options — set custom memory and launch arguments per account.',
	},
	advancedOptionsToggle: {
		id: 'app.instance.multi-launch.advanced-options-toggle',
		defaultMessage: 'Toggle advanced options for this account',
	},
	memoryLabel: {
		id: 'app.instance.multi-launch.memory-label',
		defaultMessage: 'Memory:',
	},
	memoryUnit: {
		id: 'app.instance.multi-launch.memory-unit',
		defaultMessage: 'MB',
	},
	extraArgsPlaceholder: {
		id: 'app.instance.multi-launch.extra-args-placeholder',
		defaultMessage: 'Extra JVM arguments (optional)',
	},
})

const STEVE_HEAD_URL = 'https://launcher-files.modrinth.com/assets/steve_head.png'

function avatarUrl(account: MultiLaunchAccount) {
	if (account.kind !== 'microsoft') return STEVE_HEAD_URL
	return `https://mc-heads.net/avatar/${account.id}/128`
}

const LOW_RAM_THRESHOLD_MB = 8192

const modal = ref<InstanceType<typeof NewModal>>()
const selectedIds = ref<Set<string>>(new Set())
const expandedIds = ref<Set<string>>(new Set())
const advancedMode = ref(false)
const overrides = reactive<Record<string, { memoryMb: number; extraArgs: string }>>({})

const showLowRamWarning = computed(
	() =>
		selectedIds.value.size >= 2 &&
		props.totalMemoryMb !== null &&
		props.totalMemoryMb < LOW_RAM_THRESHOLD_MB,
)

watch(
	() => props.accounts,
	(accounts) => {
		for (const account of accounts) {
			if (!overrides[account.id]) {
				overrides[account.id] = { memoryMb: 2048, extraArgs: '' }
			}
		}
	},
	{ immediate: true },
)

function toggleSelected(accountId: string) {
	const next = new Set(selectedIds.value)
	if (next.has(accountId)) {
		next.delete(accountId)
	} else {
		next.add(accountId)
	}
	selectedIds.value = next
}

function toggleExpanded(accountId: string) {
	const next = new Set(expandedIds.value)
	if (next.has(accountId)) {
		next.delete(accountId)
	} else {
		next.add(accountId)
	}
	expandedIds.value = next
}

function confirmLaunch() {
	if (selectedIds.value.size === 0) return
	const selections: MultiLaunchSelection[] = [...selectedIds.value].map((accountId) => {
		if (!advancedMode.value) return { accountId }
		const override = overrides[accountId]
		const trimmedArgs = override?.extraArgs.trim()
		return {
			accountId,
			memoryMb: override?.memoryMb,
			extraLaunchArgs: trimmedArgs ? trimmedArgs.split(/\s+/) : undefined,
		}
	})
	emit('launch', selections)
	selectedIds.value = new Set()
	modal.value?.hide()
}

function show() {
	modal.value?.show()
}

defineExpose({
	show,
})
</script>
