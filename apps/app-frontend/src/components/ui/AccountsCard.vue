<template>
	<div
		v-if="accounts.length === 0"
		class="flex flex-col gap-3 bg-button-bg border border-solid border-surface-5 rounded-xl p-3 mt-2"
	>
		<span>{{ formatMessage(messages.notSignedIn) }}</span>
		<Button type="colored" color="brand" :disabled="loginDisabled" @click="login()">
			<LogInIcon v-if="!loginDisabled" />
			<SpinnerIcon v-else class="animate-spin" />
			{{ formatMessage(messages.signInToMinecraft) }}
		</Button>
		<ButtonStyled color="secondary">
			<button @click="openOfflineModal()">
				<UserIcon />
				{{ formatMessage(messages.addOfflineAccount) }}
			</button>
		</ButtonStyled>
		<ButtonStyled color="secondary">
			<button :disabled="elyByLoginDisabled" @click="elyByLogin()">
				<GlobeIcon v-if="!elyByLoginDisabled" />
				<SpinnerIcon v-else class="animate-spin" />
				{{ formatMessage(messages.signInWithElyBy) }}
			</button>
		</ButtonStyled>
	</div>
	<Accordion
		v-else
		class="w-full mt-2 bg-button-bg border border-solid border-surface-5 rounded-xl overflow-clip"
		button-class="button-base w-full bg-transparent px-3 py-2 border-0 cursor-pointer"
		:open-by-default="false"
	>
		<template #title>
			<div class="flex gap-2 w-full min-w-0">
				<Avatar
					size="36px"
					disable-conditional-icon-padding
					:src="
						selectedAccount
							? avatarUrl
							: 'https://launcher-files.modrinth.com/assets/steve_head.png'
					"
				/>
				<div class="flex flex-col items-start w-full min-w-0">
					<span class="flex items-center gap-1 w-full min-w-0">
						<span class="truncate text-left">{{
							selectedAccount ? selectedAccount.profile.name : formatMessage(messages.selectAccount)
						}}</span>
						<span
							v-if="selectedAccount?.kind === 'elyby'"
							v-tooltip="'Ely.by'"
							class="inline-flex size-4 shrink-0 items-center justify-center rounded-full text-[9px] font-bold text-white"
							style="background-color: #00b6a5"
							>E</span
						>
						<span
							v-else-if="selectedAccount?.kind === 'offline'"
							v-tooltip="formatMessage(messages.offlineModalTitle)"
							class="inline-flex size-4 shrink-0 items-center justify-center rounded-full bg-surface-5 text-secondary"
						>
							<WifiOffIcon class="w-3 h-3" />
						</span>
					</span>
					<span class="text-secondary text-xs">{{ accountKindLabel(selectedAccount?.kind) }}</span>
				</div>
			</div>
		</template>
		<div class="bg-button-bg pt-1 pb-2 border border-solid border-surface-5">
			<template v-if="accounts.length > 0">
				<div v-for="account in accounts" :key="account.profile.id" class="flex gap-1 items-center">
					<button
						class="flex items-center flex-shrink flex-grow overflow-clip gap-2 p-2 border-0 bg-transparent cursor-pointer button-base min-w-0"
						:class="{ 'opacity-60 cursor-wait': !!switchingAccountId }"
						:disabled="!!switchingAccountId"
						@click="setAccount(account)"
					>
						<SpinnerIcon
							v-if="switchingAccountId === account.profile.id"
							class="w-5 h-5 text-brand shrink-0 animate-spin"
						/>
						<RadioButtonCheckedIcon
							v-else-if="selectedAccount && selectedAccount.profile.id === account.profile.id"
							class="w-5 h-5 text-brand shrink-0"
						/>
						<RadioButtonIcon v-else class="w-5 h-5 text-secondary shrink-0" />
						<Avatar
							:src="getAccountAvatarUrl(account)"
							size="24px"
							disable-conditional-icon-padding
						/>
						<p
							class="m-0 truncate min-w-0"
							:class="
								selectedAccount && selectedAccount.profile.id === account.profile.id
									? 'text-contrast font-semibold'
									: 'text-primary'
							"
						>
							{{ account.profile.name }}
						</p>
						<span
							v-if="account.kind === 'elyby'"
							v-tooltip="'Ely.by'"
							class="inline-flex size-4 shrink-0 items-center justify-center rounded-full text-[9px] font-bold text-white"
							style="background-color: #00b6a5"
							>E</span
						>
						<span
							v-else-if="account.kind === 'offline'"
							v-tooltip="formatMessage(messages.offlineModalTitle)"
							class="inline-flex size-4 shrink-0 items-center justify-center rounded-full bg-surface-5 text-secondary"
						>
							<WifiOffIcon class="w-3 h-3" />
						</span>
					</button>
					<IconButton
						v-tooltip="formatMessage(messages.removeAccount)"
						type="quiet"
						color="red"
						:label="formatMessage(messages.removeAccount)"
						class="mr-2 !bg-button-bg !text-primary ![box-shadow:var(--shadow-button)] hover:!bg-red focus-visible:!bg-red hover:!text-[var(--color-accent-contrast)] focus-visible:!text-[var(--color-accent-contrast)]"
						@click="logout(account.profile.id)"
					>
						<TrashIcon />
					</IconButton>
				</div>
			</template>
			<div class="flex flex-col gap-2 px-2 pt-2">
				<Button
					v-if="accounts.length > 0"
					class="w-full !bg-button-bg !text-primary ![box-shadow:var(--shadow-button)]"
					:disabled="loginDisabled"
					@click="login()"
				>
					<PlusIcon />
					{{ formatMessage(messages.addAccount) }}
				</Button>
				<ButtonStyled class="w-full" color="secondary">
					<button @click="openOfflineModal()">
						<UserIcon />
						{{ formatMessage(messages.addOfflineAccount) }}
					</button>
				</ButtonStyled>
				<ButtonStyled class="w-full" color="secondary">
					<button :disabled="elyByLoginDisabled" @click="elyByLogin()">
						<GlobeIcon v-if="!elyByLoginDisabled" />
						<SpinnerIcon v-else class="animate-spin" />
						{{ formatMessage(messages.signInWithElyBy) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</Accordion>

	<!-- Offline account modal -->
	<div
		v-if="showOfflineModal"
		class="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
		@click.self="closeOfflineModal()"
	>
		<div
			class="bg-bg border border-solid border-surface-5 rounded-xl p-6 w-80 flex flex-col gap-4 shadow-2xl"
		>
			<h3 class="m-0 text-base font-semibold text-contrast">
				{{ formatMessage(messages.offlineModalTitle) }}
			</h3>
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.offlineModalHint) }}
			</p>
			<input
				v-model="offlineUsername"
				class="w-full rounded-lg border border-solid border-surface-5 bg-button-bg px-3 py-2 text-sm text-contrast outline-none focus:border-brand"
				type="text"
				maxlength="20"
				:placeholder="formatMessage(messages.offlineUsernamePlaceholder)"
				@keydown.enter="confirmOfflineLogin()"
			/>
			<p v-if="offlineError" class="m-0 text-xs text-red">{{ offlineError }}</p>
			<div class="flex gap-2 justify-end">
				<ButtonStyled color="secondary">
					<button @click="closeOfflineModal()">
						{{ formatMessage(messages.cancel) }}
					</button>
				</ButtonStyled>
				<ButtonStyled color="brand">
					<button :disabled="offlineLoading" @click="confirmOfflineLogin()">
						<SpinnerIcon v-if="offlineLoading" class="animate-spin" />
						{{ formatMessage(messages.confirm) }}
					</button>
				</ButtonStyled>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import {
	GlobeIcon,
	LogInIcon,
	PlusIcon,
	RadioButtonCheckedIcon,
	RadioButtonIcon,
	SpinnerIcon,
	TagCategoryWifiOffIcon as WifiOffIcon,
	TrashIcon,
	UserIcon,
} from '@modrinth/assets'
import {
	Accordion,
	Avatar,
	Button,
	ButtonStyled,
	defineMessages,
	IconButton,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import type { Ref } from 'vue'
import { computed, onUnmounted, ref } from 'vue'

import { useAppEvent } from '@/composables/use-app-event'
import { handleSevereError } from '@/composables/use-error.js'
import { trackEvent } from '@/helpers/analytics'
import {
	cancel_elyby_login,
	elyby_login,
	login as login_flow,
	offline_login,
	remove_user,
	set_default_user,
	users,
} from '@/helpers/auth'
import { currentAccountId as defaultUser, refreshCurrentAccountId } from '@/helpers/current-account'
import { getPlayerHeadUrl } from '@/helpers/rendering/batch-skin-renderer.ts'
import type { Skin } from '@/helpers/skins'
import { get_available_skins } from '@/helpers/skins'

const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const emit = defineEmits<{
	change: []
}>()

type MinecraftCredential = {
	profile: {
		id: string
		name: string
	}
	kind: 'microsoft' | 'offline' | 'elyby'
}

const accounts: Ref<MinecraftCredential[]> = ref([])
const loginDisabled = ref(false)
const elyByLoginDisabled = ref(false)
const switchingAccountId = ref<string | null>(null)
const equippedSkin = ref<Skin | null>(null)
const headUrlCache = ref(new Map<string, string>())

// --- Offline login ---
const showOfflineModal = ref(false)
const offlineUsername = ref('')
const offlineError = ref('')
const offlineLoading = ref(false)

async function openOfflineModal() {
	offlineUsername.value = ''
	offlineError.value = ''
	showOfflineModal.value = true
}

function closeOfflineModal() {
	showOfflineModal.value = false
}

async function confirmOfflineLogin() {
	const name = offlineUsername.value.trim()
	if (name.length < 2 || name.length > 20) {
		offlineError.value = 'Никнейм должен быть от 2 до 20 символов'
		return
	}
	offlineLoading.value = true
	offlineError.value = ''
	try {
		const loggedIn = await offline_login(name)
		if (loggedIn) {
			await setAccount(loggedIn)
			await refreshValues()
		}
		showOfflineModal.value = false
	} catch (e) {
		offlineError.value = e instanceof Error ? e.message : 'Неизвестная ошибка'
	} finally {
		offlineLoading.value = false
	}
}
// --- /Offline login ---

async function refreshValues() {
	await refreshCurrentAccountId().catch(handleError)
	const userList = await users().catch(handleError)
	accounts.value = Array.isArray(userList) ? [...userList] : []
	accounts.value.sort((a, b) => (a.profile?.name ?? '').localeCompare(b.profile?.name ?? ''))

	try {
		const skins = await get_available_skins()
		equippedSkin.value = skins.find((skin) => skin.is_equipped) ?? null

		if (equippedSkin.value) {
			try {
				const headUrl = await getPlayerHeadUrl(equippedSkin.value)
				headUrlCache.value = new Map(headUrlCache.value).set(
					equippedSkin.value.texture_key,
					headUrl,
				)
			} catch (error) {
				console.warn('Failed to get head render for equipped skin:', error)
			}
		}
	} catch {
		equippedSkin.value = null
	}
}

async function setEquippedSkin(skin: Skin) {
	equippedSkin.value = skin

	try {
		const headUrl = await getPlayerHeadUrl(skin)
		headUrlCache.value = new Map(headUrlCache.value).set(skin.texture_key, headUrl)
	} catch (error) {
		console.warn('Failed to get head render for equipped skin:', error)
	}
}

function setLoginDisabled(value: boolean) {
	loginDisabled.value = value
}

defineExpose({
	refreshValues,
	setEquippedSkin,
	setLoginDisabled,
	login,
	loginDisabled,
})

await refreshValues()

const selectedAccount = computed(() =>
	accounts.value.find((account) => account.profile.id === defaultUser.value),
)

function accountKindLabel(kind: MinecraftCredential['kind'] | undefined) {
	switch (kind) {
		case 'elyby':
			return formatMessage(messages.elybyAccountLabel)
		case 'offline':
			return formatMessage(messages.offlineAccountLabel)
		default:
			return formatMessage(messages.minecraftAccount)
	}
}

const STEVE_HEAD_URL = 'https://launcher-files.modrinth.com/assets/steve_head.png'

const avatarUrl = computed(() => {
	if (equippedSkin.value?.texture_key) {
		const cachedUrl = headUrlCache.value.get(equippedSkin.value.texture_key)
		if (cachedUrl) {
			return cachedUrl
		}
		return `https://mc-heads.net/avatar/${equippedSkin.value.texture_key}/128`
	}
	if (selectedAccount.value?.kind === 'microsoft' && selectedAccount.value?.profile?.id) {
		return `https://mc-heads.net/avatar/${selectedAccount.value.profile.id}/128`
	}
	return STEVE_HEAD_URL
})

function getAccountAvatarUrl(account: MinecraftCredential) {
	if (
		account.profile.id === selectedAccount.value?.profile?.id &&
		equippedSkin.value?.texture_key
	) {
		const cachedUrl = headUrlCache.value.get(equippedSkin.value.texture_key)
		if (cachedUrl) {
			return cachedUrl
		}
	}
	// mc-heads.net is keyed by Mojang UUID, which offline/Ely.by accounts don't have
	if (account.kind !== 'microsoft') {
		return STEVE_HEAD_URL
	}
	return `https://mc-heads.net/avatar/${account.profile.id}/128`
}

async function setAccount(account: MinecraftCredential) {
	if (switchingAccountId.value) return
	switchingAccountId.value = account.profile.id
	equippedSkin.value = null
	try {
		await set_default_user(account.profile.id).catch(handleError)
		defaultUser.value = account.profile.id
		await refreshValues()
		emit('change')
	} finally {
		switchingAccountId.value = null
	}
}

async function login() {
	loginDisabled.value = true
	const loggedIn = await login_flow().catch(handleSevereError)

	if (loggedIn) {
		await setAccount(loggedIn)
	}

	trackEvent('AccountLogIn')
	loginDisabled.value = false
}

async function elyByLogin() {
	elyByLoginDisabled.value = true
	const loggedIn = await elyby_login().catch(handleSevereError)

	if (loggedIn) {
		await setAccount(loggedIn)
	}

	trackEvent('AccountLogIn')
	elyByLoginDisabled.value = false
}

async function logout(id: string) {
	await remove_user(id).catch(handleError)
	await refreshValues()
	if (!selectedAccount.value && accounts.value.length > 0) {
		await setAccount(accounts.value[0])
	} else {
		emit('change')
	}
	trackEvent('AccountLogOut')
}

useAppEvent('process', async (e) => {
	if (e.event === 'launched') {
		await refreshValues()
	}
})

onUnmounted(() => {
	cancel_elyby_login()
})

const messages = defineMessages({
	notSignedIn: {
		id: 'minecraft-account.not-signed-in',
		defaultMessage: 'Not signed in',
	},
	addAccount: {
		id: 'minecraft-account.add-account',
		defaultMessage: 'Add account',
	},
	removeAccount: {
		id: 'minecraft-account.remove-account',
		defaultMessage: 'Remove account',
	},
	selectAccount: {
		id: 'minecraft-account.select-account',
		defaultMessage: 'Select account',
	},
	minecraftAccount: {
		id: 'minecraft-account.label',
		defaultMessage: 'Minecraft account',
	},
	elybyAccountLabel: {
		id: 'minecraft-account.elyby-label',
		defaultMessage: 'Ely.by account',
	},
	offlineAccountLabel: {
		id: 'minecraft-account.offline-label',
		defaultMessage: 'Offline account',
	},
	signInToMinecraft: {
		id: 'minecraft-account.sign-in',
		defaultMessage: 'Sign in to Minecraft',
	},
	addOfflineAccount: {
		id: 'minecraft-account.add-offline',
		defaultMessage: 'Add offline account',
	},
	signInWithElyBy: {
		id: 'minecraft-account.sign-in-elyby',
		defaultMessage: 'Sign in with Ely.by',
	},
	offlineModalTitle: {
		id: 'minecraft-account.offline-modal.title',
		defaultMessage: 'Offline account',
	},
	offlineModalHint: {
		id: 'minecraft-account.offline-modal.hint',
		defaultMessage: 'Works only on servers with online-mode=false. No Microsoft login required.',
	},
	offlineUsernamePlaceholder: {
		id: 'minecraft-account.offline-modal.username-placeholder',
		defaultMessage: 'Enter nickname (2–20 chars)',
	},
	cancel: {
		id: 'action.cancel',
		defaultMessage: 'Cancel',
	},
	confirm: {
		id: 'action.confirm',
		defaultMessage: 'Confirm',
	},
})
</script>
