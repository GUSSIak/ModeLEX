<script setup>
import {
	AuthFeature,
	ModrinthApiError,
	NodeAuthFeature,
	nodeAuthState,
	PanelVersionFeature,
	TauriModrinthClient,
	VerboseLoggingFeature,
} from '@modrinth/api-client'
import {
	ChevronLeftIcon,
	ChevronRightIcon,
	CompassIcon,
	LogInIcon,
	LogOutIcon,
	NewspaperIcon,
	PlayIcon,
	PlusIcon,
	RefreshCwIcon,
	RightArrowIcon,
	ServerStackIcon,
	SettingsIcon,
	ShirtIcon,
	TagCategoryAudioIcon as AudioIcon,
	UserIcon,
} from '@modrinth/assets'
import {
	Admonition,
	Avatar,
	ButtonLink,
	commonMessages,
	ContentInstallModal,
	ContentUpdaterModal,
	CreationFlowModal,
	defineMessages,
	I18nDebugPanel,
	IconButton,
	IntlFormatted,
	LoadingBar,
	NewsArticleCard,
	NotificationPanel,
	PopupNotificationPanel,
	provideModalBehavior,
	provideModrinthClient,
	provideNotificationManager,
	providePageContext,
	providePopupNotificationManager,
	TeleportOverflowMenu,
	useDebugLogger,
	useFormatBytes,
	useHostingIntercom,
	useVIntl,
} from '@modrinth/ui'
import { renderString } from '@modrinth/utils'
import { useQuery, useQueryClient } from '@tanstack/vue-query'
import { getVersion } from '@tauri-apps/api/app'
import { convertFileSrc, invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { fetch as tauriFetch } from '@tauri-apps/plugin-http'
import { openUrl } from '@tauri-apps/plugin-opener'
import { type } from '@tauri-apps/plugin-os'
import { saveWindowState, StateFlags } from '@tauri-apps/plugin-window-state'
import { computed, nextTick, onMounted, onUnmounted, provide, ref, watch } from 'vue'
import { RouterView, useRoute, useRouter } from 'vue-router'

import ModLEXAppLogo from '@/assets/modlex_app.svg?component'
import AccountsCard from '@/components/ui/AccountsCard.vue'
import AppActionBar from '@/components/ui/AppActionBar.vue'
import Breadcrumbs from '@/components/ui/Breadcrumbs.vue'
import ErrorModal from '@/components/ui/ErrorModal.vue'
import FloatingAccountWidget from '@/components/ui/FloatingAccountWidget.vue'
import FriendsList from '@/components/ui/friends/FriendsList.vue'
import HostingUpdateRequired from '@/components/ui/HostingUpdateRequired.vue'
import AddServerToInstanceModal from '@/components/ui/install_flow/AddServerToInstanceModal.vue'
import UnknownPackWarningModal from '@/components/ui/install_flow/UnknownPackWarningModal.vue'
import IconEditorModal from '@/components/ui/instance_settings/icon-editor-modal/index.vue'
import MinecraftAuthErrorModal from '@/components/ui/minecraft-auth-error-modal/MinecraftAuthErrorModal.vue'
import MinecraftRequiredModal from '@/components/ui/minecraft-required-modal/MinecraftRequiredModal.vue'
import AppSettingsModal from '@/components/ui/modal/AppSettingsModal.vue'
import BetaChannelModal from '@/components/ui/modal/BetaChannelModal.vue'
import InstallToPlayModal from '@/components/ui/modal/InstallToPlayModal.vue'
import ModpackAlreadyInstalledModal from '@/components/ui/modal/ModpackAlreadyInstalledModal.vue'
import ModrinthAccountRequiredModal from '@/components/ui/modal/ModrinthAccountRequiredModal.vue'
import UpdateToPlayModal from '@/components/ui/modal/UpdateToPlayModal.vue'
import NavButton from '@/components/ui/NavButton.vue'
import NewIconEditorNotification from '@/components/ui/new-icon-editor-notification/index.vue'
import { shouldShowNewIconEditorNotification } from '@/components/ui/new-icon-editor-notification/show-notification'
import OnboardingChecklist from '@/components/ui/onboarding-checklist/index.vue'
import PrideFundraiserBanner from '@/components/ui/PrideFundraiserBanner.vue'
import QuickInstanceSwitcher from '@/components/ui/QuickInstanceSwitcher.vue'
import SharedInstanceInviteHandler from '@/components/ui/shared-instances/shared-instance-invite-handler/index.vue'
import SplashScreen from '@/components/ui/SplashScreen.vue'
import SurveyPopup from '@/components/ui/SurveyPopup.vue'
import WindowControls from '@/components/ui/WindowControls.vue'
import { useCheckDisableMouseover } from '@/composables/macCssFix.js'
import { useAppEvent } from '@/composables/use-app-event'
import { useAppSettings } from '@/composables/use-app-settings.ts'
import { useError } from '@/composables/use-error.js'
import { useTheme } from '@/composables/use-theme.ts'
import { config } from '@/config'
import { debugAnalytics, initAnalytics, trackEvent } from '@/helpers/analytics'
import { check_reachable } from '@/helpers/auth.js'
import { get_user, get_version } from '@/helpers/cache.js'
import { useFeatureFlag } from '@/helpers/feature-flags'
import { install_create_modpack_instance, install_get_modpack_preview } from '@/helpers/install'
import { can_current_user_use_shared_instances, get as getInstance, run } from '@/helpers/instance'
import {
	refreshFeatureFlags,
	startFeatureFlagPolling,
	stopFeatureFlagPolling,
} from '@/helpers/modlex-feature-flags'
import { fetchModlexNews } from '@/helpers/modlex-github-news'
import { startPing, stopPing } from '@/helpers/modlex-ping'
// ===== ModLEX IMPORTS =====
import {
	modlexFloatingGlassEffect,
	modlexHideFriends,
	modlexHideMusicTab,
	modlexHideRightSidebar,
	modlexHideServers,
	modlexNewsSource,
} from '@/helpers/modlex-settings'
import { get as getCreds, login, logout } from '@/helpers/mr_auth.ts'
import { mergeUrlQuery, parseModrinthLink } from '@/helpers/project-links.ts'
import { get as getSettings, set as setSettings } from '@/helpers/settings.ts'
import { get_opening_command, initialize_state } from '@/helpers/state'
import { hasActivePride26Midas, hasMidasBadge } from '@/helpers/user-campaigns.ts'
import { parse_modrinth_user_link } from '@/helpers/users'
import {
	areUpdatesEnabled,
	checkUpdateChannel,
	enqueueUpdateForInstallation,
	getOS,
	getUpdateSize,
	isDev,
	isNetworkMetered,
	setRestartAfterPendingUpdate,
} from '@/helpers/utils.js'
import { start_join_server, start_join_singleplayer_world } from '@/helpers/worlds.ts'
import i18n from '@/i18n.config'
import { instanceKeys } from '@/pages/instance/query-options'
import {
	appUpdateState,
	downloadAvailableAppUpdate,
	getNextAppUpdatePopupTime,
	installAvailableAppUpdate,
	markAppUpdateActionable,
	markAppUpdatePopupShown,
	openAppUpdateChangelog,
	setAppUpdateActions,
} from '@/providers/app-update.ts'
import { createBreadcrumbManager, provideBreadcrumbManager } from '@/providers/breadcrumbs'
import { createContentInstall, provideContentInstall } from '@/providers/content-install'
import {
	provideAppUpdateDownloadProgress,
	subscribeToDownloadProgress,
} from '@/providers/download-progress.ts'
import { createServerInstall, provideServerInstall } from '@/providers/server-install'
import { setupProviders } from '@/providers/setup'
import { setupAppEventsProvider } from '@/providers/setup/app-events'
import { setupAuthProvider } from '@/providers/setup/auth'
import { setupLoadingStateProvider } from '@/providers/setup/loading-state'
import { setupAppUserPreferencesProvider } from '@/providers/setup/user-preferences.ts'
import { appMessages } from '@/utils/app-messages'

// ===== END ModLEX IMPORTS =====
import { generateSkinPreviews } from './helpers/rendering/batch-skin-renderer'
import { get_available_capes, get_available_skins } from './helpers/skins'
import { AppNotificationManager } from './providers/app-notifications'
import { AppPopupNotificationManager } from './providers/app-popup-notifications'
import { appSettingsModalOpenProfileKey } from './providers/app-settings-modal'

const appSettings = useAppSettings()
const appTheme = useTheme()
const router = useRouter()
const route = useRoute()
const { channel: appEventChannel, events: appEvents } = setupAppEventsProvider()
const breadcrumbManager = createBreadcrumbManager()
provideBreadcrumbManager(breadcrumbManager)
const canNavigateBack = ref(false)
const canNavigateForward = ref(false)

function updateHistoryNavigationState() {
	const historyState = window.history.state
	canNavigateBack.value = historyState?.back != null
	canNavigateForward.value = historyState?.forward != null
}

updateHistoryNavigationState()

const APP_LEFT_NAV_WIDTH = '4rem'
const APP_SIDEBAR_WIDTH = 300
const INTERCOM_BUBBLE_DEFAULT_PADDING = 20
const PRIDE_FUNDRAISER_END_DATE = new Date('2026-07-01T00:00:00Z').getTime()
const credentials = ref()
let credentialsRefreshId = 0
const sidebarToggled = ref(true)
watch(
	() => appSettings.toggleSidebar,
	(toggleSidebar) => {
		sidebarToggled.value = !toggleSidebar
	},
)
const forceSidebar = computed(
	() =>
		route.path.startsWith('/browse') ||
		route.path.startsWith('/project') ||
		route.path.startsWith('/user'),
)
const sidebarVisible = computed(() => sidebarToggled.value || forceSidebar.value)

// ===== MODLEX: скрытие правой панели =====
// Панель всегда остаётся в DOM (нужно для #sidebar-teleport-target — Browse/
// Discover/страница проекта телепортируют туда фильтры категорий; если убрать
// панель через v-if, цель телепорта тоже исчезает и Vue падает при навигации).
// Два режима, когда modlexHideRightSidebar включён:
// — на forceSidebar-роутах (там нужны телепортированные фильтры категорий) —
//   панель "подглядывает" узкой полоской и выезжает целиком при наведении;
// — на всех остальных роутах — панель просто полностью уезжает за край без
//   намёка и без реакции на наведение, вместо неё — плавающая плашка аккаунта.
const sidebarAutoHideHovered = ref(false)
const sidebarPeekMode = computed(() => modlexHideRightSidebar.value && forceSidebar.value)
const sidebarFullyHideMode = computed(() => modlexHideRightSidebar.value && !forceSidebar.value)
const sidebarRevealed = computed(() => !sidebarPeekMode.value || sidebarAutoHideHovered.value)
const hostingRouteActive = computed(() => route.path.startsWith('/hosting'))
const hostingUpdateRequired = computed(
	() =>
		hostingRouteActive.value &&
		!!appUpdateState.availableUpdate.value &&
		appUpdateState.updatesEnabled.value,
)
const prideFundraiserEnabled = computed(
	() => appSettings.getFeatureFlag('pride_fundraiser') && Date.now() < PRIDE_FUNDRAISER_END_DATE,
)
const hostingIntercomIdentityKey = computed(() => {
	const rawServerId = route.params.id
	const serverId = Array.isArray(rawServerId) ? rawServerId[0] : rawServerId
	const userId = credentials.value?.user_id ?? credentials.value?.user?.id ?? 'anonymous'
	return `${userId}:${serverId ?? 'hosting'}`
})
const hostingIntercom = useHostingIntercom({
	enabled: computed(
		() => hostingRouteActive.value && !hostingUpdateRequired.value && !!credentials.value?.session,
	),
	appId: 'ykeritl9',
	fetchToken: fetchIntercomToken,
	identityKey: hostingIntercomIdentityKey,
	horizontalPadding: computed(() =>
		sidebarVisible.value
			? APP_SIDEBAR_WIDTH + INTERCOM_BUBBLE_DEFAULT_PADDING
			: INTERCOM_BUBBLE_DEFAULT_PADDING,
	),
})

const notificationManager = new AppNotificationManager()
provideNotificationManager(notificationManager)
const { handleError, addNotification } = notificationManager

useAppEvent(
	'warning',
	(event) =>
		addNotification({
			title: formatMessage(messages.warning),
			text: event.message,
			type: 'warning',
		}),
	appEvents,
)

const popupNotificationManager = new AppPopupNotificationManager()
providePopupNotificationManager(popupNotificationManager)
const { addPopupNotification } = popupNotificationManager

const appVersion = getVersion()
const tauriApiClient = new TauriModrinthClient({
	userAgent: async () => `modrinth/theseus/${await appVersion} (support@modrinth.com)`,
	labrinthBaseUrl: config.labrinthBaseUrl,
	archonBaseUrl: config.archonBaseUrl,
	sharedInstancesBaseUrl: config.sharedInstancesBaseUrl,
	features: [
		new NodeAuthFeature({
			getAuth: () => nodeAuthState.getAuth?.() ?? null,
			refreshAuth: async () => {
				if (nodeAuthState.refreshAuth) {
					await nodeAuthState.refreshAuth()
				}
			},
		}),
		new AuthFeature({
			token: async () => (await getCreds())?.session,
		}),
		new PanelVersionFeature(),
		new VerboseLoggingFeature(),
	],
})
provideModrinthClient(tauriApiClient)
const { data: authenticatedModrinthUser } = useQuery({
	queryKey: computed(() => ['authenticated-user', 'campaigns', credentials.value?.user?.id]),
	queryFn: () => tauriApiClient.labrinth.users_v3.getAuthenticated(),
	enabled: () => !!credentials.value?.session,
	retry: false,
})
useQuery({
	queryKey: computed(() => instanceKeys.sharedEligibility(credentials.value?.user?.id)),
	queryFn: can_current_user_use_shared_instances,
	enabled: () => !!credentials.value?.session && !!credentials.value?.user?.id,
	retry: false,
	staleTime: Infinity,
	refetchOnMount: false,
	refetchOnWindowFocus: false,
	refetchOnReconnect: false,
})
const hasPlus = computed(
	() =>
		!!credentials.value?.user &&
		(hasMidasBadge(credentials.value.user) ||
			hasActivePride26Midas(authenticatedModrinthUser.value?.campaigns?.pride_26)),
)
// MODLEX: ads removed entirely — always report no ads regardless of plan/plus status
const showAd = ref(false)
const adConsentAvailable = ref(false)
providePageContext({
	hierarchicalSidebarAvailable: ref(true),
	showAds: showAd,
	adConsentAvailable,
	floatingActionBarOffsets: {
		left: ref(APP_LEFT_NAV_WIDTH),
		right: computed(() => (sidebarVisible.value ? `${APP_SIDEBAR_WIDTH}px` : '0px')),
	},
	intercomBubble: hostingIntercom.intercomBubble,
	featureFlags: {
		serverRamAsBytesAlwaysOn: computed(() =>
			appSettings.getFeatureFlag('server_ram_as_bytes_always_on'),
		),
	},
	openExternalUrl: (url) => void openUrl(url),
})
provideModalBehavior({
	noblur: computed(() => !appTheme.advancedRendering),
})

const creationIconEditorModal = ref(null)
const creationGeneratedIcon = ref(null)
const creationIconTarget = ref('creation-flow')

const {
	installationModal,
	unknownPackWarningModal,
	fetchExistingInstanceNames,
	handleCreate,
	handleBrowseModpacks,
	searchProjects,
	getLoaderManifest,
	setModpackAlreadyInstalledModal,
	handleModpackDuplicateCreateAnyway,
	handleModpackDuplicateGoToInstance,
	onboardingChecklist,
} = setupProviders(
	tauriApiClient,
	notificationManager,
	popupNotificationManager,
	appEvents,
	(iconPath) =>
		creationGeneratedIcon.value?.path === iconPath ? creationGeneratedIcon.value.config : null,
)
const { hasLoggedIntoModrinth, showChecklist } = onboardingChecklist
const showFriendsList = computed(() => !showChecklist.value || hasLoggedIntoModrinth.value)

async function randomizeCreationIcon() {
	const generated = await creationIconEditorModal.value?.randomizeAndSave()
	if (!generated) return null

	creationGeneratedIcon.value = { path: generated.iconPath, config: generated.config }
	return {
		path: generated.iconPath,
		previewUrl: convertFileSrc(generated.iconPath),
	}
}

function customizeCreationIcon() {
	creationIconTarget.value = 'creation-flow'
	creationIconEditorModal.value?.show()
}

function customizeContentInstallIcon() {
	creationIconTarget.value = 'content-install'
	creationIconEditorModal.value?.show()
}

function onCreationIconSaved(iconPath, config) {
	creationGeneratedIcon.value = { path: iconPath, config }
	if (creationIconTarget.value === 'content-install') {
		modInstallModal.value?.setIcon(iconPath, convertFileSrc(iconPath))
		return
	}

	const context = installationModal.value?.ctx
	if (!context) return

	context.instanceIcon.value = null
	context.instanceIconUrl.value = convertFileSrc(iconPath)
	context.instanceIconPath.value = iconPath
}

// ===== MODLEX NEWS SYSTEM =====
const news = ref([])
const newsLoading = ref(false)

async function loadNews() {
	const source = modlexNewsSource.value

	// Если выключено — сразу очищаем и выходим
	if (source === 'off') {
		news.value = []
		newsLoading.value = false
		return
	}

	news.value = []
	newsLoading.value = true

	try {
		if (source === 'github') {
			const items = await fetchModlexNews()
			news.value = items.map((item) => ({
				path: item.url || '#',
				thumbnail: item.image || '',
				title: item.title || 'Без названия',
				summary: item.summary || '',
				date: item.date || '',
			}))
		} else {
			const response = await fetch('https://modrinth.com/news/feed/articles.json')
			const res = await response.json()
			if (res && res.articles) {
				news.value = res.articles
					.map((article) => ({
						...article,
						path: article.link,
					}))
					.slice(0, 4)
			}
		}
	} catch (error) {
		console.error('Failed to fetch news:', error)
		try {
			const cached = localStorage.getItem('modlex_news_fallback')
			if (cached) {
				news.value = JSON.parse(cached)
			}
		} catch {
			// ignore corrupted cache
		}
	} finally {
		await new Promise((resolve) => setTimeout(resolve, 300))
		newsLoading.value = false
	}
}
// ===== END MODLEX NEWS SYSTEM =====

const displayedServerInviteNotifications = new Set()
const serverInvitePopupNotificationIds = new Set()
let liveNotificationGeneration = 0
let liveNotificationsEnabled = true

const offline = ref(!navigator.onLine)
window.addEventListener('offline', () => {
	offline.value = true
})
window.addEventListener('online', () => {
	offline.value = false
})

const nativeDecorations = ref(false)

const os = ref('')
const isDevEnvironment = ref(false)

const stateInitialized = ref(false)

const criticalErrorMessage = ref()

const isMaximized = ref(false)

const authUnreachableDebug = useDebugLogger('AuthReachableChecker')
const authServerQuery = useQuery({
	queryKey: ['authServerReachability'],
	queryFn: async () => {
		await check_reachable()
		authUnreachableDebug('Auth servers are reachable')
		return true
	},
	refetchInterval: 5 * 60 * 1000, // 5 minutes
	retry: false,
	refetchOnWindowFocus: false,
})

const authUnreachable = computed(() => {
	if (authServerQuery.isError.value && !authServerQuery.isLoading.value) {
		console.warn('Failed to reach auth servers', authServerQuery.error.value)
		return true
	}
	return false
})

// ===== MODLEX: скрытие кнопки серверов =====
const hideServersTab = ref(false)
const hideMusicTab = ref(false)
const { enabled: musicFeatureEnabled } = useFeatureFlag('modlex_music')
// ===== END MODLEX =====

// ===== MODLEX: секретная кодовая фраза для devMode =====
// Печатается в любом месте приложения (кроме текстовых полей) — раскрывает
// скрытые в открытой бете фичи через devMode (см. helpers/feature-flags.ts).
// Специально не завязано на клики по видимому UI-элементу, чтобы не находилось
// случайно.
//
// Сравниваем по event.code (физическая клавиша), а не event.key (символ) —
// иначе при активной русской раскладке физическая "M" даёт "ь", а не "m", и
// фраза никогда не совпадёт.
let devModeSecretCodes = []
let devModeSecretBuffer = []

function isEditableTarget(target) {
	if (!(target instanceof HTMLElement)) return false
	return target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable
}

function secretToKeyCodes(secret) {
	return secret
		.split('')
		.map((char) => (/[0-9]/.test(char) ? `Digit${char}` : `Key${char.toUpperCase()}`))
}

async function toggleDevModeFromSecret() {
	appSettings.devMode = !appSettings.devMode
	const currentSettings = await getSettings()
	currentSettings.developer_mode = !!appSettings.devMode
	await setSettings(currentSettings)

	addNotification({
		title: 'ModLEX',
		text: appSettings.devMode ? 'Режим разработчика включён.' : 'Режим разработчика выключен.',
		type: appSettings.devMode ? 'success' : 'info',
	})
}

function handleDevModeSecretKeydown(event) {
	if (!devModeSecretCodes.length || isEditableTarget(event.target)) return
	// Работает только при открытых настройках — не ловит нажатия по всему приложению
	if (!appSettingsModal.value?.isSettingsOpen) return

	devModeSecretBuffer = [...devModeSecretBuffer, event.code].slice(-devModeSecretCodes.length)

	if (devModeSecretBuffer.every((code, i) => code === devModeSecretCodes[i])) {
		devModeSecretBuffer = []
		toggleDevModeFromSecret()
	}
}
// ===== END MODLEX =====

// ===== MODLEX: стартовый гейт бета-сборки =====
// Если appVersion содержит "-beta" (сборка собрана из бета-тега) и локально
// ещё нет подтверждённого modlex_beta_verified — блокируем доступ к
// приложению до ввода кода тестировщика. Отдельно от modlex_update_channel:
// тот управляет тем, какие обновления апп проверяет, а этот гейт — можно ли
// вообще пользоваться уже запущенной бета-сборкой (защита от утечки exe).
const isBetaBuild = ref(false)
const betaGateModal = ref()
const betaGateBlocking = ref(true)
const betaGateNeeded = ref(false)

async function onBetaGateApproved() {
	const currentSettings = await getSettings()
	currentSettings.modlex_beta_verified = true
	currentSettings.modlex_update_channel = 'beta'
	await setSettings(currentSettings)
	setTimeout(() => {
		betaGateBlocking.value = false
		betaGateModal.value?.hide()
	}, 1200)
}
// ===== END MODLEX =====

onMounted(async () => {
	// ===== MODLEX: подключаем слушатель секретной фразы независимо и раньше
	// всего остального — чтобы сбой в другой части onMounted (сеть и т.п.) не
	// помешал ему зарегистрироваться =====
	try {
		const appVersion = await getVersion()
		devModeSecretCodes = secretToKeyCodes(`modlex${appVersion.replace(/\D/g, '')}bgfc`)
		window.addEventListener('keydown', handleDevModeSecretKeydown)

		isBetaBuild.value = appVersion.includes('-beta')
	} catch (err) {
		console.error('[devMode secret] не удалось подключить слушатель:', err)
	}
	// ===== END MODLEX =====

	// Проверяем фича-флаги раньше всего остального — чтобы залоченные функции
	// были заблокированы уже к моменту, когда пользователь может куда-то кликнуть
	await refreshFeatureFlags()
	startFeatureFlagPolling()

	// ===== MODLEX: решение по гейту бета-сборки — после refreshFeatureFlags(),
	// чтобы учитывать актуальный remote-флаг beta_tester_gate (можно удалённо
	// отключить требование кода без новой сборки — см. flags.json). =====
	if (isBetaBuild.value && useFeatureFlag('beta_tester_gate').enabled.value) {
		try {
			const currentSettings = await getSettings()
			if (!currentSettings.modlex_beta_verified) {
				betaGateBlocking.value = true
				// Модалка живёт под v-if="stateInitialized" — до этого момента
				// её ref ещё не существует, .show() тут был бы молчаливым no-op.
				// Реальный показ — в watch(stateInitialized, ...) ниже.
				betaGateNeeded.value = true
			}
		} catch (err) {
			console.error('[beta gate] не удалось проверить состояние:', err)
		}
	}
	// ===== END MODLEX =====

	// ===== MODLEX: бета-сборка всегда сидит на бета-канале обновлений
	// (ОДНОРАЗОВАЯ коррекция, см. modlex_channel_sync_done) =====
	// modlex_update_channel обычно выставляется в onBetaGateApproved() при
	// первом прохождении гейта — но если он был пройден до того, как это
	// появилось (или значение как-то иначе осталось "stable"), бета-сборка
	// молча проверяет обновления по публичному манифесту и никогда не
	// находит новее себя. Раз бинарник и так -beta, канал обязан быть beta —
	// без этого тумблер "включить бета-канал" в настройках вводит в
	// заблуждение (он уже как бы включён самим фактом сборки).
	//
	// ВАЖНО: это должно сработать только ОДИН раз (флаг modlex_channel_sync_done)
	// — иначе при КАЖДОМ перезапуске бета-сборка молча перетирала бы ручной
	// выбор пользователя, например кнопку "Вернуться на публичный канал" в
	// настройках, которая иначе выглядела бы так, будто ничего не происходит.
	if (isBetaBuild.value) {
		try {
			const currentSettings = await getSettings()
			if (!currentSettings.modlex_channel_sync_done) {
				if (currentSettings.modlex_update_channel !== 'beta') {
					currentSettings.modlex_update_channel = 'beta'
				}
				currentSettings.modlex_channel_sync_done = true
				await setSettings(currentSettings)
			}
		} catch (err) {
			console.error('[beta channel sync] не удалось синхронизировать канал:', err)
		}
	}
	// ===== END MODLEX =====

	await useCheckDisableMouseover()

	document.querySelector('body').addEventListener('click', handleClick)
	document.querySelector('body').addEventListener('auxclick', handleAuxClick)

	checkUpdates()

	// ===== MODLEX: загружаем настройки и новости =====
	hideServersTab.value = modlexHideServers.value
	hideMusicTab.value = modlexHideMusicTab.value
	await loadNews()
	startPing()

	// Слушаем изменения настроек
	window.addEventListener('modlex-settings-changed', () => {
		hideServersTab.value = modlexHideServers.value
		hideMusicTab.value = modlexHideMusicTab.value
		loadNews()
	})
	// ===== END MODLEX =====
})

onUnmounted(async () => {
	document.querySelector('body').removeEventListener('click', handleClick)
	document.querySelector('body').removeEventListener('auxclick', handleAuxClick)
	window.removeEventListener('keydown', handleDevModeSecretKeydown)
	clearDelayedUpdatePopup()
	stopFeatureFlagPolling()
	stopPing()

	await unlistenUpdateDownload?.()
})

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()

const messages = defineMessages({
	warning: { id: 'app.notification.warning', defaultMessage: 'Warning' },
	moreOptions: { id: 'app.navigation.more-options', defaultMessage: 'More options' },
	goBack: { id: 'app.navigation.go-back', defaultMessage: 'Go back' },
	goForward: { id: 'app.navigation.go-forward', defaultMessage: 'Go forward' },
	nextImage: { id: 'app.navigation.next-image', defaultMessage: 'Next image' },
	updateDownloadMissingVersion: {
		id: 'app.update.download-error.missing-version',
		defaultMessage: 'Failed to download update: no version available',
	},
	updateInstalledToastTitle: {
		id: 'app.update.complete-toast.title',
		defaultMessage: 'Version {version} was successfully installed!',
	},
	updateInstalledToastText: {
		id: 'app.update.complete-toast.text',
		defaultMessage: 'Click here to view the changelog.',
	},
	authUnreachableHeader: {
		id: 'app.auth-servers.unreachable.header',
		defaultMessage: 'Cannot reach authentication servers',
	},
	authUnreachableBody: {
		id: 'app.auth-servers.unreachable.body',
		defaultMessage:
			'Minecraft authentication servers may be down right now. Check your internet connection and try again later.',
	},
	home: {
		id: 'app.nav.home',
		defaultMessage: 'Home',
	},
	modrinthHosting: {
		id: 'app.nav.modrinth-hosting',
		defaultMessage: 'Modrinth Hosting',
	},
	createNewInstance: {
		id: 'app.nav.create-new-instance',
		defaultMessage: 'Create new instance',
	},
	modrinthAccount: {
		id: 'app.nav.modrinth-account',
		defaultMessage: 'Modrinth account',
	},
	signedInAs: {
		id: 'app.nav.signed-in-as',
		defaultMessage: 'Signed in as <user>{username}</user>',
	},
	signInToModrinthAccount: {
		id: 'app.nav.sign-in-to-modrinth-account',
		defaultMessage: 'Sign in to a Modrinth account',
	},
	restarting: {
		id: 'app.restarting',
		defaultMessage: 'Restarting...',
	},
	upgradeToModrinthPlus: {
		id: 'app.nav.upgrade-to-modrinth-plus',
		defaultMessage: 'Upgrade to Modrinth+',
	},
	news: {
		id: 'app.news.title',
		defaultMessage: 'News',
	},
	viewAllNews: {
		id: 'app.news.view-all',
		defaultMessage: 'View all news',
	},
	playingAs: {
		id: 'app.sidebar.playing-as',
		defaultMessage: 'Playing as',
	},
})

async function setupApp() {
	await onboardingChecklist.initialize()

	if (shouldShowNewIconEditorNotification(showChecklist.value)) {
		addPopupNotification({
			contentType: 'custom',
			component: NewIconEditorNotification,
			autoCloseMs: null,
		})
	}

	const {
		native_decorations,
		theme,
		locale,
		telemetry,
		hide_nametag_skins_page,
		advanced_rendering,
		toggle_sidebar,
		sync_theme_across_devices,
		sync_behavior_across_devices,
		developer_mode,
		feature_flags,
		pending_update_toast_for_version,
	} = await getSettings()

	// Initialize locale from saved settings
	if (locale) {
		i18n.global.locale.value = locale
	}

	os.value = await getOS()
	const dev = await isDev()
	isDevEnvironment.value = dev
	const version = await getVersion()
	nativeDecorations.value = native_decorations
	if (os.value !== 'MacOS') await getCurrentWindow().setDecorations(native_decorations)

	appTheme.preferred = theme
	appTheme.advancedRendering = advanced_rendering
	appTheme.syncAcrossDevices = sync_theme_across_devices
	appSettings.syncBehaviorAcrossDevices = sync_behavior_across_devices
	appSettings.hideNametagSkinsPage = hide_nametag_skins_page
	appSettings.toggleSidebar = toggle_sidebar
	appSettings.devMode = developer_mode
	Object.assign(appSettings.featureFlags, feature_flags)
	stateInitialized.value = true

	isMaximized.value = await getCurrentWindow().isMaximized()

	await getCurrentWindow().onResized(async () => {
		isMaximized.value = await getCurrentWindow().isMaximized()
	})

	if (telemetry) {
		initAnalytics()
		if (dev) debugAnalytics()
		trackEvent('Launched', { version, dev })
	}

	if (!dev) document.addEventListener('contextmenu', (event) => event.preventDefault())

	const osType = await type()
	if (osType === 'macos') {
		document.getElementsByTagName('html')[0].classList.add('mac')
	} else {
		document.getElementsByTagName('html')[0].classList.add('windows')
	}

	fetch(`https://api.modrinth.com/appCriticalAnnouncement.json?version=${version}`)
		.then((response) => response.json())
		.then((res) => {
			if (res && res.header && res.body) {
				criticalErrorMessage.value = res
			}
		})
		.catch(() => {
			console.log(
				`No critical announcement found at https://api.modrinth.com/appCriticalAnnouncement.json?version=${version}`,
			)
		})

	// Оригинальный fetch новостей УДАЛЁН — теперь используется loadNews()

	get_opening_command().then(handleCommand)
	fetchCredentials()

	try {
		const skins = (await get_available_skins()) ?? []
		const capes = (await get_available_capes()) ?? []
		generateSkinPreviews(skins, capes)
	} catch (error) {
		console.warn('Failed to generate skin previews in app setup.', error)
	}

	if (pending_update_toast_for_version !== null) {
		const settings = await getSettings()
		settings.pending_update_toast_for_version = null
		await setSettings(settings)
	}
}

const stateFailed = ref(false)
initialize_state(appEventChannel)
	.then(() => {
		setupApp().catch((err) => {
			stateFailed.value = true
			console.error(err)
			error.showError(err, null, false, 'state_init')
		})
	})
	.catch((err) => {
		stateFailed.value = true
		console.error('Failed to initialize app', err)
		error.showError(err, null, false, 'state_init')
	})

const handleClose = async () => {
	await saveWindowState(StateFlags.ALL)
	await getCurrentWindow().close()
}

const loading = setupLoadingStateProvider()
loading.setEnabled(false)
let initialLoadToken = loading.begin()
let routerToken = null
let suspenseToken = null

let suspensePending = false

const sidebarOverlayScrollbarsOptions = Object.freeze({
	overflow: {
		x: 'hidden',
		y: 'scroll',
	},
})

router.beforeEach(() => {
	suspensePending = false
	if (routerToken) loading.end(routerToken)
	routerToken = loading.begin()
})
router.afterEach((to, from, failure) => {
	updateHistoryNavigationState()
	trackEvent('PageView', {
		path: to.path,
		fromPath: from.path,
		failed: failure,
	})
	setTimeout(() => {
		if (!suspensePending && stateInitialized.value) {
			if (initialLoadToken) {
				loading.end(initialLoadToken)
				initialLoadToken = null
			}
			if (routerToken) {
				loading.end(routerToken)
				routerToken = null
			}
		}
	}, 100)
})

function onSuspensePending() {
	suspensePending = true
	if (suspenseToken) loading.end(suspenseToken)
	suspenseToken = loading.begin()
}

function onSuspenseResolve() {
	if (suspenseToken) {
		loading.end(suspenseToken)
		suspenseToken = null
	}
	if (routerToken) {
		loading.end(routerToken)
		routerToken = null
	}
}

const queryClient = useQueryClient()

// ===== MODLEX: показ гейта бета-сборки =====
// Отдельный watch, а не проверка внутри watch(stateInitialized) ниже — тот
// срабатывает только один раз на переход false→true, а betaGateNeeded может
// стать true как до, так и после этого перехода (порядок не гарантирован,
// зависит от скорости асинхронных проверок версии/настроек). computed
// пересчитывается при изменении любого из двух флагов, так что сработает
// независимо от того, что случилось раньше.
const shouldShowBetaGate = computed(() => stateInitialized.value && betaGateNeeded.value)
watch(shouldShowBetaGate, (show) => {
	if (show) {
		betaGateNeeded.value = false
		nextTick(() => betaGateModal.value?.show())
	}
})
// ===== END MODLEX =====

watch(stateInitialized, (ready) => {
	if (ready) {
		if (initialLoadToken) {
			loading.end(initialLoadToken)
			initialLoadToken = null
		}
		if (routerToken) {
			loading.end(routerToken)
			routerToken = null
		}

		queryClient.prefetchQuery({
			queryKey: ['servers'],
			queryFn: async () => {
				const response = await tauriApiClient.archon.servers_v0.list({ limit: 100 })
				const hasMedalServers = response.servers.some((s) => s.is_medal)
				if (hasMedalServers) {
					const subscriptions = await tauriApiClient.labrinth.billing_internal.getSubscriptions()
					for (const server of response.servers) {
						if (server.is_medal) {
							const sub = subscriptions.find((s) => s.metadata?.id === server.server_id)
							if (sub) {
								server.medal_expires = new Date(
									new Date(sub.created).getTime() + 5 * 86400000,
								).toISOString()
							}
						}
					}
				}
				return response
			},
			staleTime: 30_000,
		})
		queryClient.prefetchQuery({
			queryKey: ['billing', 'subscriptions'],
			queryFn: () => tauriApiClient.labrinth.billing_internal.getSubscriptions(),
			staleTime: 30_000,
		})
		queryClient.prefetchQuery({
			queryKey: ['billing', 'payments'],
			queryFn: () => tauriApiClient.labrinth.billing_internal.getPayments(),
			staleTime: 30_000,
		})
	}
})

const error = useError()
const errorModal = ref()
const minecraftAuthErrorModal = ref()
const minecraftRequiredModal = ref()

const contentInstall = createContentInstall({ router, handleError, appEvents })
provideContentInstall(contentInstall)
const {
	instances: contentInstallInstances,
	compatibleLoaders: contentInstallLoaders,
	gameVersions: contentInstallGameVersions,
	loading: contentInstallLoading,
	defaultTab: contentInstallDefaultTab,
	preferredLoader: contentInstallPreferredLoader,
	preferredGameVersion: contentInstallPreferredGameVersion,
	releaseGameVersions: contentInstallReleaseGameVersions,
	projectInfo: contentInstallProjectInfo,
	handleInstallToInstance,
	handleCreateAndInstall,
	prepareNewInstance,
	handleNavigate: handleContentInstallNavigate,
	handleCancel: handleContentInstallCancel,
	setContentInstallModal,
	setModpackAlreadyInstalledModal: setContentInstallModpackAlreadyInstalledModal,
	handleModpackDuplicateCreateAnyway: handleContentInstallModpackDuplicateCreateAnyway,
	handleModpackDuplicateGoToInstance: handleContentInstallModpackDuplicateGoToInstance,
	setIncompatibilityWarningModal: setContentIncompatibilityWarningModal,
	incompatibilityWarningVersions: contentInstallIncompatibilityWarningVersions,
	incompatibilityWarningCurrentGameVersion: contentInstallIncompatibilityWarningCurrentGameVersion,
	incompatibilityWarningCurrentLoader: contentInstallIncompatibilityWarningCurrentLoader,
	incompatibilityWarningProjectType: contentInstallIncompatibilityWarningProjectType,
	incompatibilityWarningProjectIconUrl: contentInstallIncompatibilityWarningProjectIconUrl,
	incompatibilityWarningProjectName: contentInstallIncompatibilityWarningProjectName,
	incompatibilityWarningMessage: contentInstallIncompatibilityWarningMessage,
	incompatibilityWarningInstalling: contentInstallIncompatibilityWarningInstalling,
	handleIncompatibilityWarningInstall: handleContentInstallIncompatibilityWarningInstall,
	handleIncompatibilityWarningCancel: handleContentInstallIncompatibilityWarningCancel,
} = contentInstall

async function prepareCreationProjectInstall(projectId, projectType) {
	if (projectType === 'modpack') {
		await contentInstall.install(
			projectId,
			null,
			null,
			'CreationModalProject',
			undefined,
			(instanceId) => void router.push(`/instance/${encodeURIComponent(instanceId)}`),
		)
		return null
	}

	await prepareNewInstance(projectId)
	const info = contentInstallProjectInfo.value
	if (!info) throw new Error(`Project information is unavailable: '${projectId}'`)

	return {
		projectId,
		title: info.title,
		iconUrl: info.iconUrl,
		link: info.link,
		owner: info.owner,
		compatibleLoaders: [...contentInstallLoaders.value],
		gameVersions: [...contentInstallGameVersions.value],
		releaseGameVersions: new Set(contentInstallReleaseGameVersions.value),
	}
}

const serverInstall = createServerInstall({
	router,
	handleError,
	popupNotificationManager,
	appEvents,
})
provideServerInstall(serverInstall)
const {
	setInstallToPlayModal: setServerInstallToPlayModal,
	setUpdateToPlayModal: setServerUpdateToPlayModal,
	setAddServerToInstanceModal: setServerAddServerToInstanceModal,
	playServerProject,
} = serverInstall

const modInstallModal = ref()
const modpackAlreadyInstalledModal = ref()
const contentInstallModpackAlreadyInstalledModal = ref()
const addServerToInstanceModal = ref()
const incompatibilityWarningModal = ref()
const installToPlayModal = ref()
const sharedInstanceInviteHandler = ref()
const updateToPlayModal = ref()

const modrinthLoginModal = ref()
const appSettingsModal = ref()
provide(appSettingsModalOpenProfileKey, () => appSettingsModal.value?.showProfile())

watch(incompatibilityWarningModal, (modal) => {
	if (modal) {
		setContentIncompatibilityWarningModal(modal)
	}
})

const authProvider = setupAuthProvider(credentials, async (_redirectPath, flow, options) => {
	if (options?.showModal === false) {
		await signIn(flow)
	} else {
		await requestSignIn(flow)
	}
})

const userPreferences = setupAppUserPreferencesProvider(authProvider, notificationManager)
let userPreferencesSync = Promise.resolve()

watch(
	[userPreferences.preferences, stateInitialized],
	([preferences, initialized]) => {
		if (!preferences || !initialized) return

		userPreferencesSync = userPreferencesSync
			.then(async () => {
				const settings = await getSettings()
				const selectedTheme = preferences.appearance.auto ? 'system' : preferences.appearance.theme
				const locale = preferences.localization.locale
				const behavior = preferences.behavior
				let settingsChanged = false

				if (appTheme.syncAcrossDevices && appTheme.preferred !== selectedTheme) {
					appTheme.preferred = selectedTheme
				}
				if (i18n.global.locale.value !== locale) {
					i18n.global.locale.value = locale
				}

				if (appTheme.syncAcrossDevices && settings.theme !== selectedTheme) {
					settings.theme = selectedTheme
					settingsChanged = true
				}
				if (settings.locale !== locale) {
					settings.locale = locale
					settingsChanged = true
				}

				if (behavior && appSettings.syncBehaviorAcrossDevices) {
					const behaviorFeatureFlags = {
						worlds_in_home: behavior.show_jump_in,
						compact_instance_cards: behavior.compact_instance_cards,
						show_instance_play_time: behavior.show_play_time,
						skip_unknown_pack_warning: !behavior.warn_on_unknown_modpacks,
						skip_non_essential_warnings: behavior.skip_non_essential_warnings,
					}

					appSettings.toggleSidebar = behavior.hide_right_sidebar
					appSettings.hideNametagSkinsPage = behavior.hide_nametag
					Object.assign(appSettings.featureFlags, behaviorFeatureFlags)

					if (settings.hide_on_process_start !== behavior.minimize_app) {
						settings.hide_on_process_start = behavior.minimize_app
						settingsChanged = true
					}
					if (settings.toggle_sidebar !== behavior.hide_right_sidebar) {
						settings.toggle_sidebar = behavior.hide_right_sidebar
						settingsChanged = true
					}
					if (settings.hide_nametag_skins_page !== behavior.hide_nametag) {
						settings.hide_nametag_skins_page = behavior.hide_nametag
						settingsChanged = true
					}
					for (const [flag, value] of Object.entries(behaviorFeatureFlags)) {
						if (settings.feature_flags[flag] !== value) {
							settings.feature_flags[flag] = value
							settingsChanged = true
						}
					}
				}

				if (settingsChanged) {
					await setSettings(settings)
				}
			})
			.catch(handleError)
	},
	{ immediate: true },
)

async function validateSession(sessionToken) {
	try {
		const response = await tauriFetch(`${config.labrinthBaseUrl}/v2/user`, {
			method: 'GET',
			headers: { Authorization: sessionToken },
		})
		if (response.status === 401) return false
		return true
	} catch {
		return true
	}
}

async function fetchCredentials() {
	const hadSession = !!credentials.value?.session
	const refreshId = ++credentialsRefreshId
	credentials.value = undefined

	const creds = await getCreds().catch(handleError)
	if (refreshId !== credentialsRefreshId) return
	if (!creds && hadSession) clearLiveNotifications()

	if (creds && creds.user_id) {
		if (creds.session && !(await validateSession(creds.session))) {
			if (refreshId !== credentialsRefreshId) return

			clearLiveNotifications()
			await logout().catch(handleError)
			if (refreshId !== credentialsRefreshId) return

			credentials.value = null
			return
		}
		creds.user = await get_user(creds.user_id, 'bypass').catch(handleError)
		if (refreshId !== credentialsRefreshId) return
	}
	credentials.value = creds ?? null
	liveNotificationsEnabled = !!creds?.session
}

async function signIn(flow = 'sign-in') {
	try {
		await login(flow)
		await fetchCredentials()
	} catch (error) {
		if (
			typeof error === 'object' &&
			typeof error['message'] === 'string' &&
			error.message.includes('Login canceled')
		) {
			// Not really an error due to being a result of user interaction, show nothing
		} else {
			handleError(error)
		}
	}
}

async function requestSignIn(flow = 'sign-in') {
	await modrinthLoginModal.value?.showSigningIn(flow)
}

async function requestModrinthAuth(flow = 'sign-in') {
	await signIn(flow)
	return !!credentials.value?.session
}

async function logOut() {
	await performLogOut()
}

async function performLogOut() {
	credentialsRefreshId++
	credentials.value = undefined
	clearLiveNotifications()

	await logout().catch(handleError)
	await fetchCredentials()
}

async function fetchIntercomToken() {
	const creds = await getCreds()
	if (!creds?.session) {
		throw new Error('Not authenticated')
	}

	const params = new URLSearchParams()
	const rawServerId = route.params.id
	const serverId = Array.isArray(rawServerId) ? rawServerId[0] : rawServerId
	if (route.path.startsWith('/hosting/manage/') && typeof serverId === 'string') {
		params.set('server_id', serverId)
	}
	const query = params.size > 0 ? `?${params.toString()}` : ''

	const response = await tauriFetch(`${config.siteUrl}/api/intercom/messenger-jwt${query}`, {
		method: 'GET',
		headers: {
			Authorization: `Bearer ${creds.session}`,
		},
	})
	if (!response.ok) {
		throw new Error(`Failed to fetch Intercom token: ${response.status}`)
	}
	return await response.json()
}

onMounted(() => {
	invoke('show_window')

	error.setErrorModal(errorModal.value)
	error.setMinecraftAuthErrorModal(minecraftAuthErrorModal.value)
	error.setMinecraftRequiredModal(minecraftRequiredModal.value)

	setContentIncompatibilityWarningModal(incompatibilityWarningModal.value)
	setContentInstallModal(modInstallModal.value)
	setContentInstallModpackAlreadyInstalledModal(contentInstallModpackAlreadyInstalledModal.value)
	setModpackAlreadyInstalledModal(modpackAlreadyInstalledModal.value)
	setServerAddServerToInstanceModal(addServerToInstanceModal.value)
	setServerInstallToPlayModal(installToPlayModal.value)
	setServerUpdateToPlayModal(updateToPlayModal.value)
})

const accounts = ref(null)
provide('accountsCard', accounts)

useAppEvent('command', handleCommand, appEvents)
useAppEvent('notification', handleLiveNotification, appEvents)

async function markLiveNotificationRead(notification) {
	try {
		await tauriApiClient.labrinth.notifications_v2.markAsRead(notification.id)
	} catch (error) {
		if (error instanceof ModrinthApiError && error.statusCode === 404) {
			console.warn(`notification ${notification.id} could not be marked as read`, error)
			return
		}
		throw error
	}
}

async function respondToServerInvite(notification, action) {
	const serverId = notification.body?.server_id
	if (typeof serverId !== 'string') {
		throw new Error('Missing server ID for invite notification.')
	}

	await tauriApiClient.request(`/servers/${serverId}/invites/${action}`, {
		api: 'archon',
		version: 1,
		method: 'POST',
	})
	await markLiveNotificationRead(notification)

	return serverId
}

async function acceptServerInviteNotification(notification) {
	try {
		const serverId = await respondToServerInvite(notification, 'accept')
		await router.push(`/hosting/manage/${encodeURIComponent(serverId)}`)
		queryClient.invalidateQueries({ queryKey: ['servers'] })
	} catch (error) {
		handleError(error)
	}
}

async function declineServerInviteNotification(notification) {
	try {
		await respondToServerInvite(notification, 'decline')
	} catch (error) {
		handleError(error)
	}
}

function openServerInviteInviterProfile(inviterName) {
	if (!inviterName) return
	void router.push(`/user/${encodeURIComponent(inviterName)}`)
}

async function handleLiveNotification(notification) {
	if (!liveNotificationsEnabled || !notification?.body || notification.read) return
	if (await sharedInstanceInviteHandler.value?.handleNotification(notification)) return

	if (notification.body.type === 'server_invite') {
		if (displayedServerInviteNotifications.has(notification.id)) return

		const generation = liveNotificationGeneration
		displayedServerInviteNotifications.add(notification.id)

		const serverName =
			typeof notification.body.server_name === 'string' ? notification.body.server_name : 'a server'
		const inviterId = notification.body.invited_by
		const invitedBy =
			typeof inviterId === 'string' ? await get_user(inviterId, 'bypass').catch(() => null) : null
		if (generation !== liveNotificationGeneration) return

		const popupNotification = addPopupNotification({
			contentType: 'toast',
			title: serverName,
			type: 'server-invite',
			actorName: invitedBy?.username ?? null,
			actorAvatarUrl: invitedBy?.avatar_url ?? null,
			entityName: serverName,
			autoCloseMs: null,
			onAccept: () => acceptServerInviteNotification(notification),
			onDecline: () => declineServerInviteNotification(notification),
			onOpenActor: () => openServerInviteInviterProfile(invitedBy?.username ?? null),
		})
		serverInvitePopupNotificationIds.add(popupNotification.id)
	}
}

function clearLiveNotifications() {
	liveNotificationGeneration++
	liveNotificationsEnabled = false
	for (const id of serverInvitePopupNotificationIds) {
		popupNotificationManager.removeNotification(id)
	}
	displayedServerInviteNotifications.clear()
	serverInvitePopupNotificationIds.clear()
	sharedInstanceInviteHandler.value?.clearNotifications()
}

async function handleCommand(e) {
	if (!e) return

	if (e.event === 'RunMRPack') {
		if (e.path.endsWith('.mrpack')) {
			const location = { type: 'fromFile', path: e.path }
			const preview = await install_get_modpack_preview(location).catch(handleError)
			if (preview?.unknownFile || preview?.externalFilesInModpack.length > 0) {
				const splitPath = e.path.split(/[\\/]/)
				const fileName = splitPath ? splitPath[splitPath.length - 1] : e.path
				unknownPackWarningModal.value?.show(
					() => install_create_modpack_instance(location).then(() => undefined),
					fileName,
					preview.externalFilesInModpack,
				)
			} else {
				await install_create_modpack_instance(location).catch(handleError)
			}
			trackEvent('InstanceCreate', {
				source: 'CreationModalFileDrop',
			})
		}
	} else if (e.event === 'LaunchInstance') {
		const instance = await getInstance(e.id).catch(handleError)
		if (!instance || instance.quarantined) return

		if (e.server) {
			await start_join_server(e.id, e.server).catch(handleError)
		} else if (e.singleplayer_world) {
			await start_join_singleplayer_world(e.id, e.singleplayer_world).catch(handleError)
		} else {
			await run(e.id).catch(handleError)
		}
	} else if (e.event === 'InstallSharedInstanceInvite') {
		await sharedInstanceInviteHandler.value?.installFromInviteId(e.invite_id)
	} else if (e.event === 'InstallServer') {
		await router.push(`/project/${e.id}`)
		await playServerProject(e.id).catch(handleError)
	} else if (e.event === 'InstallVersion') {
		const version = await get_version(e.id, 'must_revalidate').catch(handleError)
		if (version) {
			await contentInstall
				.install(version.project_id, version.id, null, 'URLConfirmModal', undefined, undefined, {
					showProjectInfo: true,
				})
				.catch(handleError)
		}
	} else {
		await contentInstall
			.install(e.id, null, null, 'URLConfirmModal', undefined, undefined, { showProjectInfo: true })
			.catch(handleError)
	}
}

const appUpdateDownload = {
	progress: appUpdateState.progress,
	version: ref(),
}
let unlistenUpdateDownload

const {
	metered,
	finishedDownloading,
	downloading,
	restarting,
	availableUpdate,
	updateSize,
	updatesEnabled,
} = appUpdateState
let delayedUpdatePopupTimeout = null

const updatePopupMessages = defineMessages({
	updateAvailable: {
		id: 'app.update-popup.title',
		defaultMessage: 'Update available',
	},
	downloadComplete: {
		id: 'app.update-popup.download-complete',
		defaultMessage: 'Download complete',
	},
	meteredBody: {
		id: 'app.update-popup.body.metered',
		defaultMessage: `ModLEX App v{version} is available now! Since you're on a metered network, we didn't automatically download it.`,
	},
	downloadedBody: {
		id: 'app.update-popup.body.download-complete',
		defaultMessage: `ModLEX App v{version} has finished downloading. Reload to update now, or automatically when you close ModLEX App.`,
	},
	linuxBody: {
		id: 'app.update-popup.body.linux',
		defaultMessage:
			'ModLEX App v{version} is available. Use your package manager to update for the latest features and fixes!',
	},
	reload: {
		id: 'app.update-popup.reload',
		defaultMessage: 'Reload to update',
	},
	download: {
		id: 'app.update-popup.download',
		defaultMessage: 'Download ({size})',
	},
	changelog: {
		id: 'app.update-popup.changelog',
		defaultMessage: 'Changelog',
	},
})

function clearDelayedUpdatePopup() {
	if (delayedUpdatePopupTimeout !== null) {
		clearTimeout(delayedUpdatePopupTimeout)
		delayedUpdatePopupTimeout = null
	}
}

function getCurrentUpdatePromptStage() {
	return finishedDownloading.value ? 'downloaded' : 'available'
}

function scheduleDelayedUpdatePopup() {
	clearDelayedUpdatePopup()

	const version = availableUpdate.value?.version
	if (!version) {
		return
	}

	const nextPopupTime = getNextAppUpdatePopupTime(version, getCurrentUpdatePromptStage())
	if (nextPopupTime === null) {
		return
	}

	const delay = nextPopupTime - Date.now()
	if (delay <= 0) {
		showDelayedUpdatePopup()
		return
	}

	delayedUpdatePopupTimeout = setTimeout(showDelayedUpdatePopup, Math.min(delay, 2_147_483_647))
}

function showDelayedUpdatePopup() {
	const update = availableUpdate.value
	if (!update) {
		return
	}

	const stage = getCurrentUpdatePromptStage()
	const nextPopupTime = getNextAppUpdatePopupTime(update.version, stage)
	if (nextPopupTime === null) {
		return
	}

	if (Date.now() < nextPopupTime) {
		scheduleDelayedUpdatePopup()
		return
	}

	if (metered.value && !finishedDownloading.value) {
		addPopupNotification({
			contentType: 'standard',
			title: formatMessage(updatePopupMessages.updateAvailable),
			text: formatMessage(updatePopupMessages.meteredBody, { version: update.version }),
			type: 'info',
			autoCloseMs: null,
			buttons: [
				{
					label: formatMessage(updatePopupMessages.download, {
						size: formatBytes(updateSize.value ?? 0),
					}),
					action: () => downloadAvailableAppUpdate(),
					color: 'brand',
				},
				{
					label: formatMessage(updatePopupMessages.changelog),
					action: () => openAppUpdateChangelog(),
					keepOpen: true,
				},
			],
		})
	} else if (finishedDownloading.value) {
		addPopupNotification({
			contentType: 'standard',
			title: formatMessage(updatePopupMessages.downloadComplete),
			text: formatMessage(updatePopupMessages.downloadedBody, {
				version: update.version,
			}),
			type: 'success',
			autoCloseMs: null,
			buttons: [
				{
					label: formatMessage(updatePopupMessages.reload),
					action: () => installAvailableAppUpdate(),
					color: 'brand',
				},
				{
					label: formatMessage(updatePopupMessages.changelog),
					action: () => openAppUpdateChangelog(),
					keepOpen: true,
				},
			],
		})
	} else {
		scheduleDelayedUpdatePopup()
		return
	}

	markAppUpdatePopupShown(update.version, stage)
}

async function checkUpdates() {
	if (!(await areUpdatesEnabled())) {
		console.log('Skipping update check as updates are disabled in this build or environment')
		updatesEnabled.value = false

		if (os.value === 'Linux' && !isDevEnvironment.value) {
			checkLinuxUpdates()
			setInterval(checkLinuxUpdates, 5 * 60 * 1000)
		}
		return
	}

	async function performCheck() {
		// ===== MODLEX: резервный канал, если настройки недоступны =====
		// getSettings() читает локальную БД — если её инициализация упала
		// (например, сломанная миграция), она недоступна вообще нигде в
		// приложении. Именно в этом случае обновление особенно нужно (оно может
		// содержать фикс), так что проверка не должна зависеть от рабочей БД —
		// иначе пользователь застревает: обновиться можно только руками.
		let channel = isBetaBuild.value ? 'beta' : 'stable'
		try {
			channel = (await getSettings()).modlex_update_channel ?? channel
		} catch (err) {
			console.warn('Не удалось прочитать канал обновлений, использую дефолт:', err)
		}
		// ===== END MODLEX =====
		const update = await checkUpdateChannel(channel)
		if (!update) {
			console.log('No update available')
			return
		}

		const isExistingUpdate = update.version === availableUpdate.value?.version

		if (isExistingUpdate) {
			console.log('Update is already known')
			scheduleDelayedUpdatePopup()
			return
		}

		appUpdateDownload.progress.value = 0
		finishedDownloading.value = false
		downloading.value = false
		updateSize.value = null
		availableUpdate.value = update

		console.log(`Update ${update.version} is available.`)

		metered.value = await isNetworkMetered()
		if (!metered.value) {
			console.log('Starting download of update')
			downloadUpdate(update)
		} else {
			console.log(`Metered connection detected, not auto-downloading update.`)
			markAppUpdateActionable(update.version)
			scheduleDelayedUpdatePopup()
		}

		getUpdateSize(update.rid).then((size) => (updateSize.value = size))
	}

	// Ретрай через 5 минут планируем всегда, даже если попытка упала — иначе
	// один неудачный check_update_channel (например, сеть на секунду моргнула)
	// молча убивал бы все последующие проверки до перезапуска приложения.
	await performCheck().catch((err) => {
		console.error('Проверка обновлений не удалась:', err)
	})
	setTimeout(
		() => {
			checkUpdates()
		},
		5 /* min */ * 60 /* sec */ * 1000 /* ms */,
	)
}

async function checkLinuxUpdates() {
	try {
		const [response, currentVersion] = await Promise.all([
			fetch('https://launcher-files.modrinth.com/updates.json'),
			getVersion(),
		])
		const updates = await response.json()
		const latestVersion = updates?.version

		if (latestVersion && latestVersion !== currentVersion) {
			markAppUpdateActionable(latestVersion)
			const nextPopupTime = getNextAppUpdatePopupTime(latestVersion)
			if (nextPopupTime !== null && Date.now() >= nextPopupTime) {
				addPopupNotification({
					contentType: 'standard',
					title: formatMessage(updatePopupMessages.updateAvailable),
					text: formatMessage(updatePopupMessages.linuxBody, { version: latestVersion }),
					type: 'info',
					autoCloseMs: null,
				})
				markAppUpdatePopupShown(latestVersion)
			}
		}
	} catch (e) {
		console.error('Failed to check for updates:', e)
	}
}

async function downloadAvailableUpdate() {
	return downloadUpdate(availableUpdate.value)
}

async function downloadUpdate(versionToDownload) {
	if (!versionToDownload) {
		handleError(formatMessage(messages.updateDownloadMissingVersion))
		return
	}

	if (downloading.value || appUpdateDownload.progress.value !== 0) {
		console.error(`Update ${versionToDownload.version} already downloading`)
		return
	}

	console.log(`Downloading update ${versionToDownload.version}`)
	downloading.value = true

	try {
		enqueueUpdateForInstallation(versionToDownload.rid)
			.then(() => {
				downloading.value = false
				finishedDownloading.value = true
				unlistenUpdateDownload?.()
				unlistenUpdateDownload = null
				console.log('Finished downloading!')
				markAppUpdateActionable(versionToDownload.version, 'downloaded')
				scheduleDelayedUpdatePopup()
			})
			.catch((e) => {
				downloading.value = false
				appUpdateDownload.progress.value = 0
				handleError(e)
			})
		unlistenUpdateDownload = await subscribeToDownloadProgress(
			appEvents,
			appUpdateDownload,
			versionToDownload.version,
		)
	} catch (e) {
		downloading.value = false
		appUpdateDownload.progress.value = 0
		handleError(e)
	}
}

async function installUpdate() {
	restarting.value = true

	try {
		await setRestartAfterPendingUpdate(true)
	} catch (e) {
		restarting.value = false
		handleError(e)
		return
	}
	setTimeout(async () => {
		await handleClose()
	}, 250)
}

// ===== MODLEX: канал обновлений — принудительная перепроверка + автоустановка
// (для переключения на бета-канал из настроек, без обычного ручного шага
// "Перезапустить и обновить") =====
async function recheckAndAutoInstallUpdate() {
	const channel = (await getSettings()).modlex_update_channel ?? 'stable'
	const update = await checkUpdateChannel(channel)
	if (!update) {
		console.log('No update available on channel', channel)
		return
	}

	appUpdateDownload.progress.value = 0
	finishedDownloading.value = false
	downloading.value = true
	updateSize.value = null
	availableUpdate.value = update

	try {
		await enqueueUpdateForInstallation(update.rid)
		downloading.value = false
		finishedDownloading.value = true
		markAppUpdateActionable(update.version, 'downloaded')
		await installUpdate()
	} catch (e) {
		downloading.value = false
		handleError(e)
	}
}
// ===== END MODLEX =====

setAppUpdateActions({
	download: downloadAvailableUpdate,
	install: installUpdate,
	changelog: () => openUrl('https://modrinth.com/news/changelog?filter=app'),
	recheckAndAutoInstall: recheckAndAutoInstallUpdate,
})

async function openModrinthProjectLinkInApp(parsed) {
	const { slug, pathSuffix, url } = parsed
	const loadToken = loading.begin()
	try {
		const { id } = await tauriApiClient.labrinth.projects_v2.check(slug)
		const query = mergeUrlQuery(route.query, url)
		await router.push({
			path: `/project/${id}${pathSuffix}`,
			query,
			hash: url.hash || undefined,
		})
	} catch (err) {
		if (err instanceof ModrinthApiError && err.statusCode === 404) {
			openUrl(url.href)
		} else {
			handleError(err)
		}
	} finally {
		loading.end(loadToken)
	}
}

function handleClick(e) {
	let target = e.target
	while (target != null) {
		if (target.matches('a')) {
			if (
				target.href &&
				['http://', 'https://', 'mailto:', 'tel:'].some((v) => target.href.startsWith(v)) &&
				!target.classList.contains('router-link-active') &&
				!target.href.startsWith('http://localhost') &&
				!target.href.startsWith('https://tauri.localhost') &&
				!target.href.startsWith('http://tauri.localhost')
			) {
				const userPath = parse_modrinth_user_link(target.href)
				const parsed = parseModrinthLink(target.href)
				if (userPath) {
					void router.push(userPath)
				} else if (target.target !== '_blank' && parsed) {
					void openModrinthProjectLinkInApp(parsed)
				} else {
					openUrl(target.href)
				}
			}
			e.preventDefault()
			break
		}
		target = target.parentElement
	}
}

function handleAuxClick(e) {
	// disables middle click -> new tab
	if (e.button === 1) {
		e.preventDefault()
		// instead do a left click
		const event = new MouseEvent('click', {
			view: window,
			bubbles: true,
			cancelable: true,
		})
		e.target.dispatchEvent(event)
	}
}

provideAppUpdateDownloadProgress(appUpdateDownload)
</script>

<template>
	<SplashScreen v-if="!stateFailed" ref="splashScreen" data-tauri-drag-region />
	<div id="teleports"></div>
	<div
		v-if="stateInitialized"
		class="app-grid-layout relative"
		:class="{ 'disable-advanced-rendering': !appTheme.advancedRendering }"
	>
		<Transition name="fade">
			<div
				v-if="restarting"
				data-tauri-drag-region
				class="inset-0 fixed bg-black/80 backdrop-blur z-[200] flex items-center justify-center"
			>
				<span
					data-tauri-drag-region
					class="flex items-center gap-4 text-contrast font-semibold text-xl select-none cursor-default"
				>
					<RefreshCwIcon data-tauri-drag-region class="animate-spin w-6 h-6" />
					{{ formatMessage(messages.restarting) }}
				</span>
			</div>
		</Transition>
		<Suspense>
			<AppSettingsModal ref="appSettingsModal" />
		</Suspense>
		<BetaChannelModal
			ref="betaGateModal"
			:blocking="betaGateBlocking"
			@approved="onBetaGateApproved"
		/>
		<Suspense>
			<ModrinthAccountRequiredModal ref="modrinthLoginModal" :request-auth="requestModrinthAuth" />
		</Suspense>
		<CreationFlowModal
			ref="installationModal"
			type="instance"
			show-snapshot-toggle
			:fetch-existing-instance-names="fetchExistingInstanceNames"
			:search-projects="searchProjects"
			:prepare-project-install="prepareCreationProjectInstall"
			:create-project-install="handleCreateAndInstall"
			:get-loader-manifest="getLoaderManifest"
			:randomize-instance-icon="randomizeCreationIcon"
			:customize-instance-icon="customizeCreationIcon"
			@create="handleCreate"
			@browse-modpacks="handleBrowseModpacks"
		/>
		<IconEditorModal
			ref="creationIconEditorModal"
			:config="creationGeneratedIcon?.config"
			@saved="onCreationIconSaved"
		/>
		<UnknownPackWarningModal ref="unknownPackWarningModal" />
		<div
			class="app-grid-navbar bg-bg-raised flex flex-col p-[0.5rem] pt-0 gap-[0.25rem] w-[--left-bar-width]"
		>
			<NavButton
				v-tooltip.right="formatMessage(messages.home)"
				to="/"
				:is-primary="(route) => route.path === '/'"
				:is-subpage="
					() =>
						route.path.startsWith('/instance') ||
						((route.path.startsWith('/browse') || route.path.startsWith('/project')) &&
							route.query.i)
				"
			>
				<PlayIcon />
			</NavButton>
			<NavButton
				v-tooltip.right="formatMessage(commonMessages.discoverContentLabel)"
				to="/browse/modpack"
				:is-primary="() => route.path.startsWith('/browse') && !route.query.i && !route.query.sid"
				:is-subpage="
					(route) => route.path.startsWith('/project') && !route.query.i && !route.query.sid
				"
			>
				<CompassIcon />
			</NavButton>
			<NavButton v-tooltip.right="formatMessage(appMessages.skinSelectorLabel)" to="/skins">
				<ShirtIcon />
			</NavButton>
			<NavButton v-if="!hideMusicTab && musicFeatureEnabled" v-tooltip.right="'Музыка'" to="/music">
				<AudioIcon />
			</NavButton>
			<NavButton
				v-if="!hideServersTab"
				v-tooltip.right="formatMessage(messages.modrinthHosting)"
				to="/hosting/manage"
				:is-primary="(r) => r.path === '/hosting/manage' || r.path === '/hosting/manage/'"
				:is-subpage="
					(r) =>
						(r.path.startsWith('/hosting/manage/') && r.path !== '/hosting/manage/') ||
						((r.path.startsWith('/browse') || r.path.startsWith('/project')) && r.query.sid)
				"
			>
				<ServerStackIcon />
			</NavButton>
			<suspense>
				<QuickInstanceSwitcher />
			</suspense>
			<NavButton
				v-tooltip.right="formatMessage(messages.createNewInstance)"
				:to="() => installationModal?.show()"
				:disabled="offline"
			>
				<PlusIcon />
			</NavButton>
			<div class="flex flex-grow"></div>
			<NavButton
				v-tooltip.right="formatMessage(commonMessages.settingsLabel)"
				:to="() => appSettingsModal?.show()"
			>
				<SettingsIcon />
			</NavButton>
			<TeleportOverflowMenu
				v-if="credentials?.user"
				v-tooltip.right="formatMessage(messages.modrinthAccount)"
				type="quiet"
				size="xl"
				:label="formatMessage(messages.moreOptions)"
				:options="[
					{
						id: 'view-profile',
						label: formatMessage(messages.signedInAs, {
							username: credentials.user.username,
						}),
						action: () => router.push(`/user/${encodeURIComponent(credentials.user.username)}`),
					},
					{
						id: 'sign-out',
						label: formatMessage(commonMessages.signOutButton),
						tone: 'red',
						action: () => logOut(),
					},
				]"
				placement="right-end"
				:distance="4"
			>
				<Avatar :src="credentials?.user?.avatar_url" alt="" size="32px" circle />
				<template #view-profile>
					<UserIcon />
					<span class="inline-flex items-center gap-1">
						<IntlFormatted
							:message-id="messages.signedInAs"
							:values="{ username: credentials?.user?.username }"
						>
							<template #user="{ children }">
								<span class="inline-flex items-center gap-1 text-contrast font-semibold">
									<Avatar :src="credentials?.user?.avatar_url" alt="" size="20px" circle />
									<component :is="() => children" />
								</span>
							</template>
						</IntlFormatted>
					</span>
				</template>
				<template #sign-out>
					<LogOutIcon />
					{{ formatMessage(commonMessages.signOutButton) }}
				</template>
			</TeleportOverflowMenu>
			<NavButton
				v-else
				v-tooltip.right="formatMessage(messages.signInToModrinthAccount)"
				:to="() => requestSignIn()"
			>
				<LogInIcon class="text-brand" />
			</NavButton>
		</div>
		<div data-tauri-drag-region class="app-grid-statusbar bg-bg-raised h-[--top-bar-height] flex">
			<div data-tauri-drag-region class="flex min-w-0 flex-1 items-center overflow-hidden p-2">
				<ModLEXAppLogo class="h-7 w-auto shrink-0 text-contrast pointer-events-none" />
				<span
					v-if="isBetaBuild"
					data-tauri-drag-region
					class="ml-1.5 shrink-0 rounded-full bg-orange-highlight px-1.5 py-0.5 text-[10px] font-bold leading-none tracking-wide text-orange pointer-events-none select-none"
					>BETA</span
				>
				<div data-tauri-drag-region class="ml-2 flex shrink-0 items-center gap-2">
					<IconButton
						type="outlined"
						:label="formatMessage(messages.goBack)"
						class="!h-7 !min-w-7 !w-7 !border !border-surface-4 !p-0 !opacity-100"
						:disabled="!canNavigateBack"
						@click="router.back()"
					>
						<ChevronLeftIcon
							class="!size-4 !text-primary"
							:class="{ 'opacity-20': !canNavigateBack }"
						/>
					</IconButton>
					<IconButton
						type="outlined"
						:label="formatMessage(messages.goForward)"
						class="!h-7 !min-w-7 !w-7 !border !border-surface-4 !p-0 !opacity-100"
						:disabled="!canNavigateForward"
						@click="router.forward()"
					>
						<ChevronRightIcon
							class="!size-4 !text-primary"
							:class="{ 'opacity-20': !canNavigateForward }"
						/>
					</IconButton>
				</div>
				<Breadcrumbs />
			</div>
			<section data-tauri-drag-region class="flex shrink-0 ml-auto items-center">
				<IconButton
					v-if="!forceSidebar && appSettings.toggleSidebar && !modlexHideRightSidebar"
					:type="sidebarToggled ? 'base' : 'quiet'"
					:label="formatMessage(messages.nextImage)"
					class="mr-3 transition-transform"
					:class="{ 'rotate-180': !sidebarToggled }"
					@click="sidebarToggled = !sidebarToggled"
				>
					<RightArrowIcon />
				</IconButton>
				<div class="flex mr-3">
					<Suspense>
						<AppActionBar />
					</Suspense>
				</div>
				<WindowControls />
			</section>
		</div>
	</div>
	<div
		v-if="stateInitialized"
		class="app-contents"
		:class="{
			'sidebar-enabled': sidebarVisible && !modlexHideRightSidebar,
			'disable-advanced-rendering': !appTheme.advancedRendering,
		}"
	>
		<div class="app-viewport flex-grow router-view">
			<SurveyPopup />
			<div
				class="loading-indicator-container h-8 fixed z-50 pointer-events-none"
				:style="{
					top: 'calc(var(--top-bar-height))',
					left: 'calc(var(--left-bar-width))',
					width: 'calc(100% - var(--left-bar-width) - var(--right-bar-width))',
				}"
			>
				<LoadingBar position="absolute" />
			</div>
			<div
				v-if="appSettings.featureFlags.page_path"
				class="absolute bottom-0 left-0 m-2 bg-tooltip-bg text-tooltip-text font-semibold rounded-full px-2 py-1 text-xs z-50"
			>
				{{ route.fullPath }}
			</div>
			<div
				id="background-teleport-target"
				class="absolute h-full -z-10 rounded-tl-[--radius-xl] overflow-hidden"
				:style="{
					width: 'calc(100% - var(--right-bar-width))',
				}"
			></div>
			<Admonition
				v-if="criticalErrorMessage"
				type="critical"
				:header="criticalErrorMessage.header"
				class="m-6 mb-0"
			>
				<div
					class="markdown-body text-primary"
					v-html="renderString(criticalErrorMessage.body ?? '')"
				></div>
			</Admonition>
			<Admonition
				v-if="authUnreachable"
				type="warning"
				:header="formatMessage(messages.authUnreachableHeader)"
				class="m-6 mb-0"
			>
				{{ formatMessage(messages.authUnreachableBody) }}
			</Admonition>
			<HostingUpdateRequired v-if="hostingUpdateRequired" />
			<RouterView v-else v-slot="{ Component }">
				<template v-if="Component">
					<Suspense @pending="onSuspensePending" @resolve="onSuspenseResolve">
						<KeepAlive include="LibraryPage">
							<component :is="Component"></component>
						</KeepAlive>
					</Suspense>
				</template>
			</RouterView>
		</div>
		<div
			class="app-sidebar mt-px shrink-0 flex flex-col border-0 border-l-[1px] border-[--brand-gradient-border] border-solid"
			:class="{
				'has-plus': hasPlus,
				'app-sidebar--peek': sidebarPeekMode,
				'app-sidebar--revealed': sidebarRevealed,
				'app-sidebar--fully-hidden': sidebarFullyHideMode,
				'app-sidebar--glass': sidebarPeekMode && modlexFloatingGlassEffect,
			}"
			@mouseenter="sidebarPeekMode && (sidebarAutoHideHovered = true)"
			@mouseleave="sidebarAutoHideHovered = false"
		>
			<div
				v-overlay-scrollbars="sidebarOverlayScrollbarsOptions"
				class="app-sidebar-scrollable flex-grow shrink relative"
				:class="{ 'pb-12': !hasPlus }"
				data-overlayscrollbars-initialize
			>
				<OnboardingChecklist
					@create-instance="installationModal?.show()"
					@login-minecraft="accounts?.login()"
					@login-modrinth="signIn"
				/>
				<div id="sidebar-teleport-target" class="sidebar-teleport-content"></div>
				<div class="sidebar-default-content" :class="{ 'sidebar-enabled': sidebarVisible }">
					<div class="p-4 border-0 border-b-[1px] border-[--brand-gradient-border] border-solid">
						<h3 class="text-base text-primary font-medium m-0">
							{{ formatMessage(messages.playingAs) }}
						</h3>
						<suspense>
							<AccountsCard ref="accounts" />
						</suspense>
					</div>
					<div
						v-show="showFriendsList && !modlexHideFriends"
						class="p-4 border-0 border-b-[1px] border-[--brand-gradient-border] border-solid"
					>
						<suspense>
							<FriendsList :credentials="credentials" :sign-in="() => requestSignIn()" />
						</suspense>
					</div>
					<PrideFundraiserBanner
						v-if="prideFundraiserEnabled"
						class="p-4 border-0 border-b-[1px] border-[--brand-gradient-border] border-solid"
					/>
					<!-- ===== MODLEX NEWS ===== -->
					<div v-if="modlexNewsSource !== 'off'" class="p-4 flex flex-col items-center">
						<!-- Заголовок -->
						<h3 class="text-base mb-4 text-primary font-medium m-0 text-left w-full">News</h3>

						<!-- Состояние загрузки -->
						<div v-if="newsLoading" class="w-full flex flex-col items-center justify-center py-8">
							<div
								class="w-8 h-8 border-4 border-brand/30 border-t-brand rounded-full animate-spin"
							></div>
							<p class="text-secondary text-sm mt-3">Загрузка новостей...</p>
						</div>

						<!-- Новости -->
						<div
							v-else-if="news && news.length > 0"
							class="space-y-4 flex flex-col items-center w-full"
						>
							<NewsArticleCard
								v-for="(item, index) in news"
								:key="`news-${index}`"
								:article="item"
							/>
							<ButtonLink
								type="colored"
								color="brand"
								size="xl"
								:href="
									modlexNewsSource === 'github'
										? 'https://github.com/gussiak/ModeLEX/releases'
										: 'https://modrinth.com/news'
								"
								target="_blank"
								class="my-4"
							>
								<NewspaperIcon />
								{{
									modlexNewsSource === 'github'
										? 'View all releases'
										: formatMessage(messages.viewAllNews)
								}}
							</ButtonLink>
						</div>

						<!-- Нет новостей (только если не off) -->
						<div v-else class="text-secondary text-sm py-4 text-center w-full">
							Нет доступных новостей
						</div>
					</div>
					<!-- ===== END MODLEX NEWS ===== -->
				</div>
			</div>
		</div>
		<FloatingAccountWidget v-if="sidebarFullyHideMode" />
	</div>
	<I18nDebugPanel />
	<NotificationPanel :has-sidebar="sidebarVisible" />
	<PopupNotificationPanel :has-sidebar="sidebarVisible" />
	<ErrorModal ref="errorModal" />
	<MinecraftAuthErrorModal ref="minecraftAuthErrorModal" />
	<MinecraftRequiredModal ref="minecraftRequiredModal" />
	<ContentInstallModal
		ref="modInstallModal"
		:instances="contentInstallInstances"
		:compatible-loaders="contentInstallLoaders"
		:game-versions="contentInstallGameVersions"
		:loading="contentInstallLoading"
		:default-tab="contentInstallDefaultTab"
		:preferred-loader="contentInstallPreferredLoader"
		:preferred-game-version="contentInstallPreferredGameVersion"
		:release-game-versions="contentInstallReleaseGameVersions"
		:project-info="contentInstallProjectInfo"
		:randomize-icon="randomizeCreationIcon"
		:customize-icon="customizeContentInstallIcon"
		@install="handleInstallToInstance"
		@create-and-install="handleCreateAndInstall"
		@navigate="handleContentInstallNavigate"
		@cancel="handleContentInstallCancel"
	/>
	<ModpackAlreadyInstalledModal
		ref="modpackAlreadyInstalledModal"
		@create-anyway="handleModpackDuplicateCreateAnyway"
		@go-to-instance="handleModpackDuplicateGoToInstance"
	/>
	<AddServerToInstanceModal ref="addServerToInstanceModal" />
	<ContentUpdaterModal
		ref="incompatibilityWarningModal"
		mode="incompatibility-warning"
		:versions="contentInstallIncompatibilityWarningVersions"
		:current-game-version="contentInstallIncompatibilityWarningCurrentGameVersion"
		:current-loader="contentInstallIncompatibilityWarningCurrentLoader"
		current-version-id=""
		:is-app="true"
		:project-type="contentInstallIncompatibilityWarningProjectType"
		:project-icon-url="contentInstallIncompatibilityWarningProjectIconUrl"
		:project-name="contentInstallIncompatibilityWarningProjectName"
		:warning="contentInstallIncompatibilityWarningMessage"
		:action-loading="contentInstallIncompatibilityWarningInstalling"
		@update="handleContentInstallIncompatibilityWarningInstall"
		@cancel="handleContentInstallIncompatibilityWarningCancel"
	/>
	<ModpackAlreadyInstalledModal
		ref="contentInstallModpackAlreadyInstalledModal"
		@create-anyway="handleContentInstallModpackDuplicateCreateAnyway"
		@go-to-instance="handleContentInstallModpackDuplicateGoToInstance"
	/>
	<SharedInstanceInviteHandler ref="sharedInstanceInviteHandler" />
	<InstallToPlayModal ref="installToPlayModal" :show-external-warnings="false" />
	<UpdateToPlayModal ref="updateToPlayModal" :show-external-warnings="false" />
</template>

<style lang="scss" scoped>
.app-grid-layout,
.app-contents {
	--top-bar-height: 3rem;
	--left-bar-width: 4rem;
	--right-bar-width: 300px;
}

.app-grid-layout {
	display: grid;
	grid-template: 'status status' 'nav dummy';
	grid-template-columns: auto 1fr;
	grid-template-rows: auto 1fr;
	position: relative;
	//z-index: 0;
	background-color: var(--color-raised-bg);
	height: 100vh;
}

.app-grid-navbar {
	grid-area: nav;
	position: relative;
	z-index: 2;
}

.app-grid-statusbar {
	grid-area: status;
	padding-right: var(--window-controls-width, 0px);
	position: relative;
	z-index: 2;
}

[data-tauri-drag-region-exclude] {
	-webkit-app-region: no-drag;
}

.app-contents {
	position: absolute;
	z-index: 1;
	left: var(--left-bar-width);
	top: var(--top-bar-height);
	right: 0;
	bottom: 0;
	height: calc(100vh - var(--top-bar-height));
	background-color: var(--color-bg);
	border-top-left-radius: var(--radius-xl);
	display: grid;
	grid-template-columns: 1fr 0px;
	// transition: grid-template-columns 0.4s ease-in-out;

	&.sidebar-enabled {
		grid-template-columns: 1fr 300px;
	}
	// Анимация для новостей
	.news-enter-active {
		transition: all 0.3s ease-out;
	}

	.news-enter-from {
		opacity: 0;
		transform: translateY(-10px);
	}
}

.loading-indicator-container {
	border-top-left-radius: var(--radius-xl);
	overflow: hidden;
}

.app-sidebar {
	overflow: visible;
	width: 300px;
	position: relative;
	height: calc(100vh - var(--top-bar-height));
	background: var(--brand-gradient-bg);

	--color-button-bg: var(--brand-gradient-button);
	--color-button-bg-hover: var(--brand-gradient-border);
	--color-divider: var(--brand-gradient-border);
	--color-divider-dark: var(--brand-gradient-border);
}

.app-sidebar::after {
	content: '';
	position: absolute;
	bottom: 0px;
	left: 0;
	right: 0;
	height: 15rem;
	background: var(--brand-gradient-fade-out-color);
	pointer-events: none;
}

.app-sidebar.has-plus::after {
	display: none;
}

/* ===== MODLEX: скрытие правой панели =====
   Панель остаётся в потоке DOM (нужно для #sidebar-teleport-target — иначе
   Discover/страница проекта роняют Vue при навигации, когда их телепорт
   внезапно теряет цель). Два режима:
   — "peek": узкая полоска-подглядывание, выезжает целиком при наведении
     (только на Discover/проекте/пользователе — там нужны фильтры категорий);
   — "fully-hidden": полностью за краем экрана, без намёка и без наведения
     (везде остальное) — вместо неё показывается FloatingAccountWidget.vue. */
.app-sidebar--peek {
	position: fixed;
	top: var(--top-bar-height);
	right: 0;
	z-index: 35;
	transform: translateX(calc(100% - 0.875rem));
	transition: transform 0.25s ease-out;
	box-shadow: -4px 0 24px rgba(0, 0, 0, 0.35);
}

.app-sidebar--peek.app-sidebar--revealed {
	transform: translateX(0);
}

.app-sidebar--fully-hidden {
	position: fixed;
	top: var(--top-bar-height);
	right: 0;
	transform: translateX(100%);
	pointer-events: none;
}

.app-sidebar--glass {
	/* --brand-gradient-bg — это linear-gradient(...), color-mix() с ним не
	   работает, поэтому для стекла берём сплошной surface-цвет вместо него. */
	background: color-mix(in srgb, var(--surface-3) 55%, transparent);
	backdrop-filter: blur(20px) saturate(160%);
	-webkit-backdrop-filter: blur(20px) saturate(160%);
}
/* ===== END MODLEX ===== */

.disable-advanced-rendering {
	.app-sidebar::before {
		box-shadow: none;
	}

	&.app-contents::before {
		box-shadow: none;
	}

	*,
	:deep(*) {
		box-shadow: none !important;
		--tw-drop-shadow:;
	}
}

.app-sidebar::before {
	content: '';
	box-shadow: -15px 0 15px -15px rgba(0, 0, 0, 0.1) inset;
	top: 0;
	bottom: 0;
	left: -2rem;
	width: 2rem;
	position: absolute;
	pointer-events: none;
}

.app-viewport {
	flex-grow: 1;
	height: 100%;
	overflow: auto;
	overflow-x: hidden;
	scrollbar-gutter: stable;
}

.app-contents::before {
	z-index: 30;
	content: '';
	position: fixed;
	left: var(--left-bar-width);
	top: var(--top-bar-height);
	right: calc(-1 * var(--left-bar-width));
	bottom: calc(-1 * var(--left-bar-width));
	border-radius: var(--radius-xl);
	box-shadow: 1px 1px 15px rgba(0, 0, 0, 0.1) inset;
	border-color: var(--surface-5);
	border-width: 1px;
	border-style: solid;
	pointer-events: none;
}

.sidebar-teleport-content {
	display: contents;
}

.sidebar-default-content {
	display: none;
}

.sidebar-teleport-content:empty + .sidebar-default-content.sidebar-enabled {
	display: contents;
}

@media (prefers-reduced-motion: no-preference) {
	.nav-button-animated-enter-active {
		transition: all 0.5s cubic-bezier(0.15, 1.4, 0.64, 0.96);
	}

	.nav-button-animated-leave-active {
		transition: all 0.25s ease;
	}

	.nav-button-animated-enter-active {
		position: relative;
	}

	.nav-button-animated-enter-active::before {
		content: '';
		inset: 0;
		border-radius: 100vw;
		background-color: var(--color-brand-highlight);
		position: absolute;
		animation: pop 0.5s ease-in forwards;
		opacity: 0;
	}

	@keyframes pop {
		0% {
			scale: 0.5;
		}
		50% {
			opacity: 0.5;
		}
		100% {
			scale: 1.5;
		}
	}

	.nav-button-animated-enter-from {
		scale: 0.5;
		translate: -2rem 0;
		opacity: 0;
	}

	.nav-button-animated-leave-to {
		scale: 0.75;
		opacity: 0;
	}

	.fade-enter-active {
		transition: 0.25s ease-in-out;
	}

	.fade-enter-from {
		opacity: 0;
	}
}
</style>
<style>
.os-theme-dark,
.os-theme-light {
	--os-handle-bg: var(--color-scrollbar) !important;
	--os-handle-bg-hover: var(--color-scrollbar) !important;
	--os-handle-bg-active: var(--color-scrollbar) !important;
}

.mac {
	.app-grid-statusbar {
		padding-left: 5rem;
	}
}

.windows {
	.fake-appbar {
		height: 2.5rem !important;
	}

	.info-card {
		right: 22rem;
	}

	.profile-card {
		right: 8rem;
	}
}

/* ===== MODLEX: двойная обводка (расширенная кастомизация цвета) =====
   Добавляет доп. кольцо поверх стандартных border/divider-цветов, не влияя
   на layout (box-shadow вместо border). Ограничено элементами с ЛИТЕРАЛЬНЫМ
   классом border/border-2/4/8 (обводка со всех 4 сторон) — иначе кольцо
   рисуется и на элементах вроде TabbedModal.vue, у которых border-divider
   стоит рядом с border-0 + border-r-[1px] (только ОДНА сторона), и получается
   лишний прямоугольник-"хитбокс" вокруг всей панели. Круглые/пилюльные
   элементы (кнопки бокового меню, аватарки) тоже исключены — двойное кольцо
   на rounded-full даёт не обводку, а сплошной ободок-кляксу. Включается
   тумблером в настройках — см. modlex-settings.ts::applyDoubleBorder. */
:root[data-modlex-double-border]
	:is([class~='border'], [class~='border-2'], [class~='border-4'], [class~='border-8']):is(
		[class*='border-surface-4'],
		[class*='border-surface-5'],
		[class*='border-divider']
	):not([class*='rounded-full']):not([class*='border-0']) {
	box-shadow:
		inset 0 0 0 1px var(--modlex-border-inner-color, #fff),
		0 0 0 1px var(--modlex-border-outer-color, #000);
}

/* Та же обводка — для иконок (по одному правилу на весь документ вместо
   тысяч селекторов, за счёт каскада на svg). */
:root[data-modlex-double-border] svg {
	filter: drop-shadow(0 0 1px var(--modlex-border-outer-color, #000));
}

/* Обводка текста — отдельная настройка (свой тумблер и цвет), не привязана
   к двойной обводке карточек. См. modlex-settings.ts::applyTextOutline. */
:root[data-modlex-text-outline] body {
	text-shadow:
		-1px -1px 0 var(--modlex-text-outline-color, #000),
		1px -1px 0 var(--modlex-text-outline-color, #000),
		-1px 1px 0 var(--modlex-text-outline-color, #000),
		1px 1px 0 var(--modlex-text-outline-color, #000);
}
/* ===== END MODLEX ===== */
</style>
