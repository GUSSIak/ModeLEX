<template>
	<StackedAdmonitions v-bind="$attrs" :items="stackItems" class="w-full">
		<template #item="{ item, dismissible }">
			<InstanceAdmonitionsSharedInstanceStale
				v-if="item.kind === 'shared-instance-stale'"
				:instance="instance"
				@published="emit('published')"
			/>
			<InstanceAdmonitionsSharedInstanceWrongAccount
				v-else-if="item.kind === 'shared-instance-wrong-account'"
				:expected-user-id="sharedInstanceExpectedUserId"
				:role="sharedInstanceRole"
				:signed-out="sharedInstanceSignedOut"
			/>
			<InstanceAdmonitionsSharedInstanceUnavailable
				v-else-if="item.kind === 'shared-instance-unavailable'"
				:reason="displayedSharedInstanceUnavailableReason"
				:manager="sharedInstanceUnavailableManager"
				:dismissible="dismissible"
				@dismiss="sharedInstanceUnavailableDismissed = true"
				@delete="emit('delete')"
			/>
			<InstanceAdmonitionsOfflineMultiplayerVersionQuirk
				v-else-if="item.kind === 'offline-multiplayer-version-quirk'"
			/>
		</template>
	</StackedAdmonitions>
</template>

<script setup lang="ts">
import { StackedAdmonitions } from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'

import { users } from '@/helpers/auth'
import { currentAccountId } from '@/helpers/current-account'
import type { SharedInstanceUnavailableReason } from '@/helpers/install'
import type { GameInstance } from '@/helpers/types'

import InstanceAdmonitionsOfflineMultiplayerVersionQuirk from './offline-multiplayer-version-quirk.vue'
import InstanceAdmonitionsSharedInstanceStale from './shared-instance-stale.vue'
import InstanceAdmonitionsSharedInstanceUnavailable from './shared-instance-unavailable.vue'
import InstanceAdmonitionsSharedInstanceWrongAccount from './shared-instance-wrong-account.vue'
import type { InstanceAdmonitionItem, SharedInstanceRole } from './types.ts'

// ModLEX: versions known to hide the Multiplayer button for offline accounts
// if the internet is reachable while the game starts up (see the toggle in
// Settings -> ModLEX -> "Запуск" for an experimental automatic workaround).
const OFFLINE_MULTIPLAYER_QUIRK_VERSIONS = new Set(['1.16.5'])

defineOptions({
	inheritAttrs: false,
})

const props = defineProps<{
	instance: GameInstance
	sharedInstanceUnavailableReason?: SharedInstanceUnavailableReason | null
	sharedInstanceUnavailableManager?: string | null
	sharedInstanceWrongAccount?: boolean
	sharedInstanceExpectedUserId?: string | null
	sharedInstanceRole?: SharedInstanceRole | null
	sharedInstanceSignedOut?: boolean
}>()

const emit = defineEmits<{
	published: []
	delete: []
}>()

// ModLEX: "is the currently playing-as account offline or Ely.by" for the
// multiplayer-version-quirk admonition below — the game is always launched
// with --userType msa regardless of account kind (see launcher/mod.rs), so
// neither a true offline account nor an Ely.by one (whose access token is
// real, but issued by Ely.by, not Microsoft) can pass Mojang's real session
// check, and it's that check being reachable that seems to trip up 1.16.5.
const accountsList = ref<Awaited<ReturnType<typeof users>>>([])
onMounted(async () => {
	accountsList.value = await users().catch(() => [])
})
const currentAccountKindAffected = computed(() => {
	const kind = accountsList.value.find(
		(account) => account.profile.id === currentAccountId.value,
	)?.kind
	return kind === 'offline' || kind === 'elyby'
})
const showOfflineMultiplayerQuirkAdmonition = computed(
	() =>
		currentAccountKindAffected.value &&
		OFFLINE_MULTIPLAYER_QUIRK_VERSIONS.has(props.instance.game_version),
)

const sharedInstanceWrongAccount = computed(() => props.sharedInstanceWrongAccount ?? false)
const displayedSharedInstanceUnavailableReason = computed<SharedInstanceUnavailableReason | null>(
	() =>
		props.instance.quarantined ? 'quarantined' : (props.sharedInstanceUnavailableReason ?? null),
)
const sharedInstanceUnavailableDismissed = ref(false)
const showSharedInstancePublishAdmonition = computed(
	() =>
		!sharedInstanceWrongAccount.value &&
		props.instance.install_stage === 'installed' &&
		props.instance.shared_instance?.role === 'owner' &&
		props.instance.shared_instance.status === 'stale',
)
const stackItems = computed<InstanceAdmonitionItem[]>(() => {
	const items: InstanceAdmonitionItem[] = []

	if (sharedInstanceWrongAccount.value) {
		items.push({
			id: 'shared-instance-wrong-account',
			type: 'warning',
			dismissible: false,
			kind: 'shared-instance-wrong-account',
		})
	}

	const unavailableReason = displayedSharedInstanceUnavailableReason.value
	const sharedInstanceQuarantined = unavailableReason === 'quarantined'
	if (
		unavailableReason &&
		(sharedInstanceQuarantined || !sharedInstanceUnavailableDismissed.value)
	) {
		items.push({
			id: 'shared-instance-unavailable',
			type: 'warning',
			dismissible: !sharedInstanceQuarantined,
			kind: 'shared-instance-unavailable',
		})
	}

	if (showSharedInstancePublishAdmonition.value) {
		items.push({
			id: 'shared-instance-stale',
			type: 'warning',
			dismissible: false,
			kind: 'shared-instance-stale',
		})
	}

	if (showOfflineMultiplayerQuirkAdmonition.value) {
		items.push({
			id: 'offline-multiplayer-version-quirk',
			type: 'info',
			dismissible: true,
			kind: 'offline-multiplayer-version-quirk',
		})
	}

	return items
})

watch(
	() => [props.instance.id, displayedSharedInstanceUnavailableReason.value],
	() => {
		sharedInstanceUnavailableDismissed.value = false
	},
)
</script>
