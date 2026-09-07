<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" max-width="600px">
		<Admonition
			v-if="showLowRamWarning"
			type="warning"
			class="mb-3"
			:header="formatMessage(messages.lowRamWarningHeader)"
			:body="formatMessage(messages.lowRamWarningBody)"
		/>

		<div
			v-if="accounts.length > 0"
			class="overflow-clip rounded-[20px] border border-solid border-surface-4"
		>
			<div class="flex h-11 items-center gap-3 bg-surface-3 px-3">
				<Checkbox
					:model-value="allSelected"
					:indeterminate="selectedIds.size > 0 && !allSelected"
					:aria-label="formatMessage(messages.selectAll)"
					class="shrink-0"
					@update:model-value="toggleSelectAll"
				/>
				<span class="min-w-0 flex-1 text-sm font-semibold text-secondary">
					{{ formatMessage(messages.accountColumn) }}
				</span>
				<span class="hidden w-[190px] shrink-0 text-sm font-semibold text-secondary sm:block">
					{{ formatMessage(messages.uuidColumn) }}
				</span>
				<span
					v-if="advancedMode"
					class="w-8 shrink-0 text-right text-sm font-semibold text-secondary"
				>
				</span>
			</div>

			<div v-for="(account, index) in accounts" :key="account.id">
				<div
					class="flex w-full items-center gap-3 px-3 py-2"
					:class="[
						index % 2 === 1 ? 'bg-surface-1.5' : 'bg-surface-2',
						index > 0 ? 'border-0 border-t border-solid border-surface-4' : '',
					]"
				>
					<Checkbox
						:model-value="selectedIds.has(account.id)"
						class="min-w-0 flex-1"
						@update:model-value="toggleSelected(account.id)"
					>
						<span class="flex min-w-0 flex-1 items-center gap-2.5">
							<Avatar :src="avatarUrl(account)" size="2.5rem" />
							<span class="flex min-w-0 flex-col items-start text-left">
								<span class="w-full truncate font-semibold leading-5 text-contrast">{{
									account.name
								}}</span>
								<span class="flex items-center gap-1 text-xs text-secondary">
									<WifiOffIcon v-if="account.kind === 'offline'" class="size-3" />
									{{ formatMessage(kindLabels[account.kind]) }}
								</span>
							</span>
						</span>
					</Checkbox>

					<button
						v-tooltip="
							formatMessage(copiedId === account.id ? messages.uuidCopied : messages.uuidCopy)
						"
						type="button"
						class="hidden shrink-0 items-center gap-1.5 rounded-lg border-0 bg-transparent px-1.5 py-1 font-mono text-xs text-secondary cursor-pointer hover:bg-surface-5 hover:text-contrast sm:flex"
						@click="copyUuid(account.id)"
					>
						{{ account.id }}
						<CheckIcon v-if="copiedId === account.id" class="size-3 shrink-0" />
						<CopyIcon v-else class="size-3 shrink-0" />
					</button>

					<OnlineIndicatorIcon
						v-if="runningAccountIds.has(account.id)"
						v-tooltip="formatMessage(messages.alreadyRunning)"
						class="shrink-0"
					/>
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
					class="flex flex-col gap-2.5 bg-surface-1 p-3"
					:class="index > 0 ? 'border-0 border-t border-solid border-surface-4' : ''"
				>
					<div class="flex flex-wrap items-center gap-x-4 gap-y-2">
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

						<div class="flex items-center gap-2">
							<label class="text-xs text-secondary">
								{{ formatMessage(messages.resolutionLabel) }}
							</label>
							<input
								v-model.number="overrides[account.id].width"
								type="number"
								min="0"
								:placeholder="formatMessage(messages.resolutionWidthPlaceholder)"
								class="w-20 rounded-lg border border-solid border-surface-5 bg-button-bg px-2 py-1 text-sm text-contrast outline-none focus:border-brand"
							/>
							<span class="text-xs text-secondary">×</span>
							<input
								v-model.number="overrides[account.id].height"
								type="number"
								min="0"
								:placeholder="formatMessage(messages.resolutionHeightPlaceholder)"
								class="w-20 rounded-lg border border-solid border-surface-5 bg-button-bg px-2 py-1 text-sm text-contrast outline-none focus:border-brand"
							/>
						</div>

						<div class="flex items-center gap-2">
							<label class="text-xs text-secondary" :for="`fullscreen-${account.id}`">
								{{ formatMessage(messages.fullscreenLabel) }}
							</label>
							<select
								:id="`fullscreen-${account.id}`"
								v-model="overrides[account.id].fullscreen"
								class="rounded-lg border border-solid border-surface-5 bg-button-bg px-2 py-1 text-sm text-contrast outline-none focus:border-brand"
							>
								<option value="default">{{ formatMessage(messages.fullscreenDefault) }}</option>
								<option value="windowed">{{ formatMessage(messages.fullscreenWindowed) }}</option>
								<option value="fullscreen">
									{{ formatMessage(messages.fullscreenFullscreen) }}
								</option>
							</select>
						</div>
					</div>

					<input
						v-model="overrides[account.id].extraArgs"
						type="text"
						class="w-full rounded-lg border border-solid border-surface-5 bg-button-bg px-2 py-1 text-sm text-contrast outline-none focus:border-brand"
						:placeholder="formatMessage(messages.extraArgsPlaceholder)"
					/>
				</div>
			</div>
		</div>
		<p v-else class="text-sm text-secondary m-0">
			{{ formatMessage(messages.noAccounts) }}
		</p>

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
	CheckIcon,
	CopyIcon,
	DropdownIcon,
	OnlineIndicatorIcon,
	PlayIcon,
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
	gameResolution?: [number, number]
	forceFullscreen?: boolean
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
	selectAll: {
		id: 'app.instance.multi-launch.select-all',
		defaultMessage: 'Select all',
	},
	accountColumn: {
		id: 'app.instance.multi-launch.account-column',
		defaultMessage: 'Account',
	},
	uuidColumn: {
		id: 'app.instance.multi-launch.uuid-column',
		defaultMessage: 'UUID',
	},
	uuidCopy: {
		id: 'app.instance.multi-launch.uuid-copy',
		defaultMessage: 'Copy UUID',
	},
	uuidCopied: {
		id: 'app.instance.multi-launch.uuid-copied',
		defaultMessage: 'Copied!',
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
	kindMicrosoft: {
		id: 'app.instance.multi-launch.kind-microsoft',
		defaultMessage: 'Microsoft',
	},
	kindOffline: {
		id: 'app.instance.multi-launch.kind-offline',
		defaultMessage: 'Offline',
	},
	kindElyby: {
		id: 'app.instance.multi-launch.kind-elyby',
		defaultMessage: 'Ely.by',
	},
	advancedModeHint: {
		id: 'app.instance.multi-launch.advanced-mode-hint',
		defaultMessage: 'Advanced options — set custom memory, resolution and launch arguments per account.',
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
	resolutionLabel: {
		id: 'app.instance.multi-launch.resolution-label',
		defaultMessage: 'Window:',
	},
	resolutionWidthPlaceholder: {
		id: 'app.instance.multi-launch.resolution-width-placeholder',
		defaultMessage: 'Width',
	},
	resolutionHeightPlaceholder: {
		id: 'app.instance.multi-launch.resolution-height-placeholder',
		defaultMessage: 'Height',
	},
	fullscreenLabel: {
		id: 'app.instance.multi-launch.fullscreen-label',
		defaultMessage: 'Display:',
	},
	fullscreenDefault: {
		id: 'app.instance.multi-launch.fullscreen-default',
		defaultMessage: 'Default',
	},
	fullscreenWindowed: {
		id: 'app.instance.multi-launch.fullscreen-windowed',
		defaultMessage: 'Windowed',
	},
	fullscreenFullscreen: {
		id: 'app.instance.multi-launch.fullscreen-fullscreen',
		defaultMessage: 'Fullscreen',
	},
	extraArgsPlaceholder: {
		id: 'app.instance.multi-launch.extra-args-placeholder',
		defaultMessage: 'Extra JVM arguments (optional)',
	},
})

const kindLabels = {
	microsoft: messages.kindMicrosoft,
	offline: messages.kindOffline,
	elyby: messages.kindElyby,
} as const

const STEVE_HEAD_URL = 'https://launcher-files.modrinth.com/assets/steve_head.png'

function avatarUrl(account: MultiLaunchAccount) {
	if (account.kind !== 'microsoft') return STEVE_HEAD_URL
	return `https://mc-heads.net/avatar/${account.id}/128`
}

const LOW_RAM_THRESHOLD_MB = 8192

type FullscreenChoice = 'default' | 'windowed' | 'fullscreen'
interface AccountOverride {
	memoryMb: number
	extraArgs: string
	width?: number
	height?: number
	fullscreen: FullscreenChoice
}

// MODLEX: per-account advanced launch settings are remembered across modal
// opens (keyed by account, not instance) — someone's "this account needs
// 6 GB" is a fact about their PC, not about any one modpack, so re-typing it
// per instance every launch would just be annoying.
const OVERRIDES_STORAGE_KEY = 'modlex-multi-launch-overrides'

function loadStoredOverrides(): Record<string, AccountOverride> {
	try {
		const raw = localStorage.getItem(OVERRIDES_STORAGE_KEY)
		return raw ? JSON.parse(raw) : {}
	} catch {
		return {}
	}
}

function persistOverrides() {
	try {
		localStorage.setItem(OVERRIDES_STORAGE_KEY, JSON.stringify(overrides))
	} catch {
		// Storage unavailable or full — advanced settings just won't be remembered.
	}
}

const modal = ref<InstanceType<typeof NewModal>>()
const selectedIds = ref<Set<string>>(new Set())
const expandedIds = ref<Set<string>>(new Set())
const advancedMode = ref(false)
const overrides = reactive<Record<string, AccountOverride>>({})
const copiedId = ref<string | null>(null)

const showLowRamWarning = computed(
	() =>
		selectedIds.value.size >= 2 &&
		props.totalMemoryMb !== null &&
		props.totalMemoryMb < LOW_RAM_THRESHOLD_MB,
)

watch(
	() => props.accounts,
	(accounts) => {
		const stored = loadStoredOverrides()
		for (const account of accounts) {
			if (!overrides[account.id]) {
				overrides[account.id] = stored[account.id] ?? {
					memoryMb: 2048,
					extraArgs: '',
					width: undefined,
					height: undefined,
					fullscreen: 'default',
				}
			}
		}
	},
	{ immediate: true },
)

const allSelected = computed(
	() => props.accounts.length > 0 && selectedIds.value.size === props.accounts.length,
)

function toggleSelectAll(checked: boolean) {
	selectedIds.value = checked ? new Set(props.accounts.map((a) => a.id)) : new Set()
}

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

async function copyUuid(accountId: string) {
	await navigator.clipboard.writeText(accountId)
	copiedId.value = accountId
	setTimeout(() => {
		if (copiedId.value === accountId) copiedId.value = null
	}, 1500)
}

function confirmLaunch() {
	if (selectedIds.value.size === 0) return
	const selections: MultiLaunchSelection[] = [...selectedIds.value].map((accountId) => {
		if (!advancedMode.value) return { accountId }
		const override = overrides[accountId]
		const trimmedArgs = override?.extraArgs.trim()
		const gameResolution: [number, number] | undefined =
			override?.width && override?.height ? [override.width, override.height] : undefined
		const forceFullscreen =
			override?.fullscreen === 'default' ? undefined : override?.fullscreen === 'fullscreen'
		return {
			accountId,
			memoryMb: override?.memoryMb,
			extraLaunchArgs: trimmedArgs ? trimmedArgs.split(/\s+/) : undefined,
			gameResolution,
			forceFullscreen,
		}
	})
	if (advancedMode.value) persistOverrides()
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
