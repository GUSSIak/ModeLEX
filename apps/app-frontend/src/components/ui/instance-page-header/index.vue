<template>
	<PageHeader :title="instance.name">
		<template #leading>
			<Avatar :src="iconSrc" :alt="instance.name" size="64px" :tint-by="instance.id" />
		</template>

		<template v-if="instance.shared_instance || instance.quarantined" #badges>
			<PageHeaderBadgeItem
				v-if="instance.quarantined"
				:icon="LockIcon"
				aria-label="Locked instance information"
				class="!border-orange !bg-highlight-orange !text-orange"
			>
				Locked
			</PageHeaderBadgeItem>
			<PageHeaderBadgeItem
				v-else
				:tooltip="sharedInstanceTooltip"
				aria-label="Shared instance information"
				class="!border-blue !bg-highlight-blue !text-blue"
			>
				Shared
				<UnknownIcon class="block size-4 shrink-0 text-current" aria-hidden="true" />
			</PageHeaderBadgeItem>
		</template>

		<template #metadata>
			<div v-if="isServerInstance" class="flex flex-wrap items-center gap-2">
				<InstanceHeaderServerMetadata
					:loading-server-ping="loadingServerPing"
					:players-online="playersOnline"
					:status-online="statusOnline"
					:recent-plays="recentPlays"
					:ping="ping"
					:minecraft-server="minecraftServer"
					:linked-project-v3="linkedProjectV3"
					:instance-id="instance.id"
				/>
				<PageHeaderMetadataItem v-if="sharedInstanceManager" :action="sharedInstanceManagerAction">
					{{ sharedInstanceManagerLabel }}
					<Avatar
						:src="sharedInstanceManager.avatarUrl"
						:alt="sharedInstanceManager.name"
						:tint-by="sharedInstanceManager.tintBy"
						size="24px"
						:circle="sharedInstanceManager.type === 'user'"
						no-shadow
					/>
					<span class="min-w-0 truncate">{{ sharedInstanceManager.name }}</span>
				</PageHeaderMetadataItem>
			</div>
			<PageHeaderMetadata v-else>
				<PageHeaderMetadataItem :icon="Gamepad2Icon" tooltip="Minecraft version">
					Minecraft {{ instance.game_version }}
				</PageHeaderMetadataItem>
				<PageHeaderMetadataItem
					v-if="sharedInstanceManager?.type !== 'user'"
					:icon="ServerLoaderIcon"
					:icon-props="{ loader: loaderDisplayName }"
					tooltip="Mod loader"
				>
					{{ loaderLabel }}
				</PageHeaderMetadataItem>
				<PageHeaderMetadataItem
					v-if="showInstancePlayTime"
					:icon="TimerIcon"
					tooltip="Total playtime"
				>
					{{ playtimeLabel }}
				</PageHeaderMetadataItem>
				<PageHeaderMetadataItem v-if="sharedInstanceManager" :action="sharedInstanceManagerAction">
					{{ sharedInstanceManagerLabel }}
					<Avatar
						:src="sharedInstanceManager.avatarUrl"
						:alt="sharedInstanceManager.name"
						:tint-by="sharedInstanceManager.tintBy"
						size="24px"
						:circle="sharedInstanceManager.type === 'user'"
						no-shadow
					/>
					<span class="min-w-0 truncate">{{ sharedInstanceManager.name }}</span>
				</PageHeaderMetadataItem>
			</PageHeaderMetadata>
		</template>

		<template #actions>
			<PageHeaderActions>
				<ButtonStyled
					v-if="
						!modlexHideMultiLaunch &&
						multiLaunchFeatureEnabled &&
						accounts.length > 1 &&
						!isInstalling &&
						!instance.quarantined &&
						instance.install_stage === 'installed'
					"
					circular
					size="large"
					type="transparent"
				>
					<button
						v-tooltip="formatMessage(messages.launchMultiple)"
						type="button"
						:aria-label="formatMessage(messages.launchMultiple)"
						@click="emit('openMultiLaunch')"
					>
						<UsersIcon />
					</button>
				</ButtonStyled>
				<ButtonStyled v-if="isInstalling" color="brand" size="large">
					<button type="button" disabled>
						{{ formatMessage(commonMessages.installingLabel) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-else-if="playing" color="red" size="large">
					<button type="button" :disabled="stopping" @click="emit('stop')">
						<StopCircleIcon />
						{{
							stopping ? formatMessage(messages.stopping) : formatMessage(commonMessages.stopButton)
						}}
					</button>
				</ButtonStyled>
				<ButtonStyled v-else-if="instance.quarantined" color="brand" size="large">
					<button v-tooltip="formatMessage(messages.lockedPlayTooltip)" type="button" disabled>
						<PlayIcon />
						{{ formatMessage(commonMessages.playButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-else-if="instance.install_stage !== 'installed'" color="brand" size="large">
					<button type="button" @click="emit('repair')">
						<DownloadIcon />
						{{ formatMessage(messages.repair) }}
					</button>
				</ButtonStyled>
				<JoinedButtons
					v-else-if="!loading && isServerInstance"
					:actions="serverPlayActions"
					color="brand"
					size="large"
				/>
				<ButtonStyled v-else-if="!loading" color="brand" size="large">
					<button type="button" @click="emit('play')">
						<PlayIcon />
						{{ formatMessage(commonMessages.playButton) }}
					</button>
				</ButtonStyled>
				<ButtonStyled v-else color="brand" size="large">
					<button type="button" disabled>{{ formatMessage(messages.starting) }}</button>
				</ButtonStyled>

				<Dropdown
					v-if="runningAccounts.length > 0"
					placement="bottom-end"
					:triggers="['click']"
					:hide-triggers="['click']"
				>
					<ButtonStyled circular size="large" type="transparent">
						<button
							v-tooltip="formatMessage(messages.runningAccounts, { count: runningAccounts.length })"
							type="button"
							:aria-label="formatMessage(messages.runningAccounts, { count: runningAccounts.length })"
						>
							{{ runningAccounts.length }}
						</button>
					</ButtonStyled>
					<template #popper>
						<div class="flex w-[18rem] flex-col gap-2 p-1">
							<div
								v-for="account in runningAccounts"
								:key="account.uuid"
								class="flex items-center gap-2 rounded-xl bg-surface-4 p-2 text-sm"
							>
								<OnlineIndicatorIcon class="shrink-0" />
								<div class="mr-auto min-w-0">
									<div class="text-contrast truncate">{{ account.accountName }}</div>
									<div class="text-secondary text-xs">{{ account.elapsedLabel }}</div>
								</div>
								<button
									v-tooltip="formatMessage(commonMessages.stopButton)"
									class="active:scale-95 flex shrink-0"
									type="button"
									:aria-label="formatMessage(commonMessages.stopButton)"
									@click="emit('stopProcess', account.uuid)"
								>
									<StopCircleIcon class="text-red size-5" />
								</button>
							</div>
						</div>
					</template>
				</Dropdown>
				<ButtonStyled circular size="large">
					<button
						v-tooltip="formatMessage(messages.instanceSettings)"
						type="button"
						:aria-label="formatMessage(messages.instanceSettings)"
						@click="emit('settings')"
					>
						<SettingsIcon />
					</button>
				</ButtonStyled>
				<ButtonStyled circular size="large" type="transparent">
					<TeleportOverflowMenu
						:options="moreActions"
						:tooltip="formatMessage(messages.moreActions)"
						:aria-label="formatMessage(messages.moreActions)"
					>
						<MoreVerticalIcon />
					</TeleportOverflowMenu>
				</ButtonStyled>
			</PageHeaderActions>
		</template>
	</PageHeader>
</template>

<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import {
	DownloadIcon,
	ExternalIcon,
	FolderOpenIcon,
	LockIcon,
	MoreVerticalIcon,
	OnlineIndicatorIcon,
	PackageIcon,
	PlayIcon,
	ReportIcon,
	SettingsIcon,
	StopCircleIcon,
	TagCategoryGamepad2Icon as Gamepad2Icon,
	TimerIcon,
	UnknownIcon,
	UsersIcon,
} from '@modrinth/assets'
import {
	Avatar,
	ButtonStyled,
	commonMessages,
	defineMessages,
	formatLoaderLabel,
	type JoinedButtonAction,
	JoinedButtons,
	LoaderIcon as ServerLoaderIcon,
	PageHeader,
	PageHeaderActions,
	PageHeaderBadgeItem,
	PageHeaderMetadata,
	PageHeaderMetadataItem,
	type ServerLoader,
	TeleportOverflowMenu,
	type TeleportOverflowMenuItem,
	useVIntl,
} from '@modrinth/ui'
import { Dropdown } from 'floating-vue'
import { computed } from 'vue'
import { useRouter } from 'vue-router'

import { useFeatureFlag } from '@/helpers/feature-flags'
import { modlexHideMultiLaunch } from '@/helpers/modlex-settings'
import type { GameInstance } from '@/helpers/types'

import InstanceHeaderServerMetadata from './instance-page-header-server-metadata.vue'

const messages = defineMessages({
	createShortcut: {
		id: 'instance.action.create-shortcut',
		defaultMessage: 'Create shortcut',
	},
	exportModpack: {
		id: 'instance.action.export-modpack',
		defaultMessage: 'Export modpack',
	},
	instanceSettings: {
		id: 'instance.action.settings',
		defaultMessage: 'Instance settings',
	},
	launchInstance: {
		id: 'instance.action.launch-instance',
		defaultMessage: 'Launch instance',
	},
	moreActions: {
		id: 'instance.action.more-actions',
		defaultMessage: 'More actions',
	},
	neverPlayed: {
		id: 'instance.playtime.never-played',
		defaultMessage: 'Never played',
	},
	openFolder: {
		id: 'instance.action.open-folder',
		defaultMessage: 'Open folder',
	},
	repair: {
		id: 'instance.action.repair',
		defaultMessage: 'Repair',
	},
	runningAccounts: {
		id: 'instance.action.running-accounts',
		defaultMessage: '{count, plural, one {# account running} other {# accounts running}}',
	},
	launchMultiple: {
		id: 'instance.action.launch-multiple',
		defaultMessage: 'Launch as multiple accounts',
	},
	lockedPlayTooltip: {
		id: 'instance.locked.play-tooltip',
		defaultMessage: 'This instance has been locked',
	},
	starting: {
		id: 'instance.action.starting',
		defaultMessage: 'Starting...',
	},
	stopping: {
		id: 'instance.action.stopping',
		defaultMessage: 'Stopping...',
	},
	sharedInstanceTooltip: {
		id: 'instance.shared-instance.tooltip',
		defaultMessage: "This instance's content is being managed by someone else.",
	},
	sharedInstanceOwnerTooltip: {
		id: 'instance.shared-instance.owner-tooltip',
		defaultMessage: "This instance's content is being shared to other users.",
	},
})
const router = useRouter()

const props = withDefaults(
	defineProps<{
		instance: GameInstance
		iconSrc?: string | null
		isServerInstance?: boolean
		showInstancePlayTime?: boolean
		timePlayed?: number
		playing?: boolean
		loading?: boolean
		stopping?: boolean
		loadingServerPing?: boolean
		playersOnline?: number
		statusOnline?: boolean
		recentPlays?: number
		ping?: number
		minecraftServer?: Labrinth.Projects.v3.Project['minecraft_server']
		linkedProjectV3?: Labrinth.Projects.v3.Project
		sharedInstanceManager?: {
			type: 'user' | 'server'
			name: string
			avatarUrl?: string
			tintBy: string
		} | null
		runningAccounts?: Array<{ uuid: string; accountName: string; elapsedLabel: string }>
		accounts?: Array<{ id: string; name: string }>
	}>(),
	{
		iconSrc: null,
		isServerInstance: false,
		showInstancePlayTime: false,
		timePlayed: 0,
		playing: false,
		loading: false,
		stopping: false,
		loadingServerPing: false,
		playersOnline: undefined,
		statusOnline: false,
		recentPlays: undefined,
		ping: undefined,
		minecraftServer: undefined,
		linkedProjectV3: undefined,
		sharedInstanceManager: null,
		runningAccounts: () => [],
		accounts: () => [],
	},
)

const emit = defineEmits<{
	repair: []
	stop: []
	play: []
	playServer: []
	settings: []
	openFolder: []
	export: []
	createShortcut: []
	report: [event?: MouseEvent]
	stopProcess: [uuid: string]
	openMultiLaunch: []
}>()

const installingStages = [
	'installing',
	'pack_installing',
	'pack_installed',
	'not_installed',
	'minecraft_installing',
]

const { formatMessage } = useVIntl()

const isInstalling = computed(() => installingStages.includes(props.instance.install_stage))
const { enabled: multiLaunchFeatureEnabled } = useFeatureFlag('multi_account_launch')
const loaderDisplayName = computed(() => formatLoaderLabel(props.instance.loader) as ServerLoader)
const loaderLabel = computed(() =>
	[loaderDisplayName.value, props.instance.loader_version].filter(Boolean).join(' '),
)
const sharedInstanceTooltip = computed(() =>
	formatMessage(
		props.instance.shared_instance?.role === 'owner'
			? messages.sharedInstanceOwnerTooltip
			: messages.sharedInstanceTooltip,
	),
)
const sharedInstanceManagerLabel = computed(() =>
	props.sharedInstanceManager?.type === 'server' ? 'Linked to' : 'Managed by',
)
const sharedInstanceManagerAction = computed(() => {
	const manager = props.sharedInstanceManager
	if (manager?.type !== 'user') return undefined
	return () => router.push(`/user/${encodeURIComponent(manager.name)}`)
})
const playtimeLabel = computed(() => {
	if (props.timePlayed <= 0) return formatMessage(messages.neverPlayed)

	const hours = Math.floor(props.timePlayed / 3600)
	if (hours >= 1) {
		return `${hours} hour${hours > 1 ? 's' : ''}`
	}

	const minutes = Math.floor(props.timePlayed / 60)
	if (minutes >= 1) {
		return `${minutes} minute${minutes > 1 ? 's' : ''}`
	}

	const seconds = Math.floor(props.timePlayed)
	return `${seconds} second${seconds > 1 ? 's' : ''}`
})
const serverPlayActions = computed<JoinedButtonAction[]>(() => [
	{
		id: 'join_server',
		label: formatMessage(commonMessages.playButton),
		icon: PlayIcon,
		action: () => emit('playServer'),
	},
	{
		id: 'launch_instance',
		label: formatMessage(messages.launchInstance),
		icon: PlayIcon,
		action: () => emit('play'),
	},
])
const moreActions = computed<TeleportOverflowMenuItem[]>(() => {
	const actions: TeleportOverflowMenuItem[] = [
		{
			id: 'open-folder',
			label: formatMessage(messages.openFolder),
			icon: FolderOpenIcon,
			action: () => emit('openFolder'),
		},
	]

	if (!props.instance.quarantined) {
		actions.push(
			{
				id: 'export-mrpack',
				label: formatMessage(messages.exportModpack),
				icon: PackageIcon,
				action: () => emit('export'),
			},
			{
				id: 'create-shortcut',
				label: formatMessage(messages.createShortcut),
				icon: ExternalIcon,
				action: () => emit('createShortcut'),
			},
		)
	}

	if (props.instance.shared_instance?.role === 'member') {
		actions.push(
			{ divider: true },
			{
				id: 'report-shared-instance',
				label: formatMessage(commonMessages.reportButton),
				icon: ReportIcon,
				color: 'red',
				action: (event) => emit('report', event),
			},
		)
	}

	return actions
})
</script>
