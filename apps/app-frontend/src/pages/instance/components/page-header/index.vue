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
			<InstanceHeaderServerMetadata
				v-if="isServerInstance"
				:loading-server-ping="loadingServerPing"
				:players-online="playersOnline"
				:status-online="statusOnline"
				:ping="ping"
				:minecraft-server="minecraftServer"
				:show-instance-play-time="showInstancePlayTime"
				:playtime-label="playtimeLabel ?? formatMessage(messages.neverPlayed)"
			/>
			<PageHeaderMetadata v-else>
				<PageHeaderMetadataItem
					:icon="TagIcon"
					:icon-props="{ tag: loaderDisplayName, enforceType: 'loader' }"
					tooltip="Mod loader and Minecraft version"
				>
					{{ loaderLabel }}
				</PageHeaderMetadataItem>
				<PageHeaderMetadataItem
					v-if="showInstancePlayTime && playtimeLabel"
					:icon="TimerIcon"
					tooltip="Total playtime"
				>
					{{ playtimeLabel }}
				</PageHeaderMetadataItem>
				<PageHeaderMetadataTimeItem
					v-if="instance.last_played"
					:icon="ClockIcon"
					:date="instance.last_played"
					:label="formatMessage(messages.lastPlayed)"
				/>
				<PageHeaderMetadataItem v-else :icon="ClockIcon" tooltip="Last played">
					{{ formatMessage(messages.neverPlayed) }}
				</PageHeaderMetadataItem>
			</PageHeaderMetadata>
		</template>

		<template #actions>
			<PageHeaderActions>
				<IconButton
					v-if="
						!modlexHideMultiLaunch &&
						multiLaunchFeatureEnabled &&
						accounts.length > 1 &&
						!isInstalling &&
						!instance.quarantined &&
						instance.install_stage === 'installed'
					"
					v-tooltip="formatMessage(messages.launchMultiple)"
					size="xl"
					:label="formatMessage(messages.launchMultiple)"
					native-type="button"
					@click="emit('openMultiLaunch')"
				>
					<UsersIcon />
				</IconButton>
				<Button
					v-if="isInstalling"
					type="colored"
					color="brand"
					size="xl"
					native-type="button"
					disabled
				>
					{{ formatMessage(commonMessages.installingLabel) }}
				</Button>
				<Button
					v-else-if="playing"
					type="colored"
					color="red"
					size="xl"
					native-type="button"
					:disabled="stopping"
					@click="emit('stop')"
				>
					<StopCircleIcon />
					{{
						stopping ? formatMessage(messages.stopping) : formatMessage(commonMessages.stopButton)
					}}
				</Button>
				<Button
					v-else-if="instance.quarantined"
					v-tooltip="formatMessage(messages.lockedPlayTooltip)"
					type="colored"
					color="brand"
					size="xl"
					native-type="button"
					disabled
				>
					<PlayIcon />
					{{ formatMessage(commonMessages.playButton) }}
				</Button>
				<Button
					v-else-if="instance.install_stage !== 'installed'"
					type="colored"
					color="brand"
					size="xl"
					native-type="button"
					@click="emit('repair')"
				>
					<DownloadIcon />
					{{ formatMessage(messages.repair) }}
				</Button>
				<SplitButton
					v-else-if="!loading && isServerInstance"
					type="colored"
					color="brand"
					size="xl"
					:options="serverPlayOptions"
					:menu-label="formatMessage(messages.launchInstance)"
					@click="emit('playServer')"
				>
					<PlayIcon />
					{{ formatMessage(commonMessages.playButton) }}
				</SplitButton>
				<Button
					v-else-if="!loading"
					type="colored"
					color="brand"
					size="xl"
					native-type="button"
					@click="emit('play')"
				>
					<PlayIcon />
					{{ formatMessage(commonMessages.playButton) }}
				</Button>
				<Button v-else type="colored" color="brand" size="xl" native-type="button" disabled>{{
					formatMessage(messages.starting)
				}}</Button>

				<Dropdown
					v-if="runningAccounts.length > 0"
					placement="bottom-end"
					:triggers="['click']"
					:hide-triggers="['click']"
				>
					<IconButton
						v-tooltip="formatMessage(messages.runningAccounts, { count: runningAccounts.length })"
						size="xl"
						:label="formatMessage(messages.runningAccounts, { count: runningAccounts.length })"
						native-type="button"
					>
						{{ runningAccounts.length }}
					</IconButton>
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
				<IconButton
					v-tooltip="formatMessage(messages.instanceSettings)"
					size="xl"
					:label="formatMessage(messages.instanceSettings)"
					native-type="button"
					@click="emit('settings')"
				>
					<SettingsIcon />
				</IconButton>
				<TeleportOverflowMenu
					type="quiet"
					size="xl"
					:label="formatMessage(messages.moreActions)"
					:tooltip="formatMessage(messages.moreActions)"
					:options="moreActions"
				>
					<MoreVerticalIcon />
				</TeleportOverflowMenu>
			</PageHeaderActions>
		</template>
	</PageHeader>
</template>

<script setup lang="ts">
import type { Labrinth } from '@modrinth/api-client'
import {
	ClockIcon,
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
	TimerIcon,
	UnknownIcon,
	UsersIcon,
} from '@modrinth/assets'
import { Button, IconButton, SplitButton, TeleportOverflowMenu } from '@modrinth/ui'
import {
	Avatar,
	commonMessages,
	defineMessages,
	formatLoaderLabel,
	type OverflowMenuOption,
	PageHeader,
	PageHeaderActions,
	PageHeaderBadgeItem,
	PageHeaderMetadata,
	PageHeaderMetadataItem,
	PageHeaderMetadataTimeItem,
	type ServerLoader,
	TagIcon,
	useVIntl,
} from '@modrinth/ui'
import { Dropdown } from 'floating-vue'
import { computed } from 'vue'

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
	lastPlayed: {
		id: 'instance.last-played',
		defaultMessage: 'Last played',
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
	[loaderDisplayName.value, props.instance.game_version].filter(Boolean).join(' '),
)
const sharedInstanceTooltip = computed(() =>
	formatMessage(
		props.instance.shared_instance?.role === 'owner'
			? messages.sharedInstanceOwnerTooltip
			: messages.sharedInstanceTooltip,
	),
)
const playtimeLabel = computed(() => {
	const seconds = Math.floor(props.timePlayed)
	if (seconds <= 0) {
		return undefined
	}

	const hours = Math.floor(seconds / 3600)
	if (hours >= 1) {
		return `${hours} hour${hours > 1 ? 's' : ''}`
	}

	const minutes = Math.floor(seconds / 60)
	if (minutes >= 1) {
		return `${minutes} minute${minutes > 1 ? 's' : ''}`
	}

	return `${seconds} second${seconds === 1 ? '' : 's'}`
})
const serverPlayOptions = computed<OverflowMenuOption[]>(() => [
	{
		id: 'launch_instance',
		label: formatMessage(messages.launchInstance),
		icon: PlayIcon,
		action: () => emit('play'),
	},
])
const moreActions = computed<OverflowMenuOption[]>(() => {
	const actions: OverflowMenuOption[] = [
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
			{ type: 'divider' },
			{
				id: 'report-shared-instance',
				label: formatMessage(commonMessages.reportButton),
				icon: ReportIcon,
				tone: 'red',
				action: (event) => emit('report', event),
			},
		)
	}

	return actions
})
</script>
