<!-- ModLEX: плавающий чат ИИ-агента лаунчера — правый нижний угол, скрывается
     через Настройки → ModLEX → Внешний вид (см. modlexHideAiAgent). Общается
     через packages/app-lib/src/api/modlex_ai.rs (пул бесплатных ключей z.ai →
     Cloudflare → Groq, с фоллбэком на собственный ключ z.ai пользователя из
     настроек ниже).

     Стейта диалога на сервере нет: `history` — это ВСЯ переписка (включая
     tool_calls/tool-результаты), она целиком уходит на бэк на каждый ход и
     целиком же приходит обратно (см. modlex_ai_agent_step в helpers/modlex-ai.ts).
     Хранится только в памяти компонента — при перезапуске лаунчера теряется.

     Фаза 1: read-only тир-0 инструменты, выполняются сами, без подтверждения.
     Фаза 2: тир-Confirm инструменты (install/remove/toggle/сменить версию
     мода, Java/память инстанции, установить Java) — ничего не выполняется,
     пока пользователь не нажмёт "Подтвердить" прямо в пузыре вызова (см.
     pendingCall/confirmPending ниже) — modlex_ai_agent_resume в helpers/modlex-ai.ts. -->
<script setup lang="ts">
import {
	BotIcon,
	CheckIcon,
	ChevronDownIcon,
	ChevronRightIcon,
	KeyIcon,
	RefreshCwIcon,
	SendIcon,
	SettingsIcon,
	StopCircleIcon,
	WrenchIcon,
	XIcon,
} from '@modrinth/assets'
import { Button, ConfirmModal, DropdownSelect, IconButton, StyledInput, Toggle } from '@modrinth/ui'
import type { UnlistenFn } from '@tauri-apps/api/event'
import { listen } from '@tauri-apps/api/event'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import { useAppSettings } from '@/composables/use-app-settings'
import { pendingAiAgentHelpRequest } from '@/composables/use-ai-agent-bridge'
import { get as getInstance } from '@/helpers/instance'
import {
	type AgentStepOutcome,
	type AiChatMessage,
	type AiToolCall,
	type KeyPingResult,
	modlex_ai_agent_resume,
	modlex_ai_agent_step,
	modlex_ai_dev_ping_available,
	modlex_ai_list_models,
	modlex_ai_ping_key_pool,
	modlex_ai_ping_own_key,
} from '@/helpers/modlex-ai'
import { get as getSettings, set as setSettings } from '@/helpers/settings'

const appSettings = useAppSettings()

const route = useRoute()
// Пока пользователь на странице конкретной инстанции — сразу знаем, о какой
// инстанции речь, без "уточни, пожалуйста, о какой из них" в начале диалога.
const currentInstanceId = computed<string | null>(() => {
	const name = route.name?.toString() ?? ''
	return name.startsWith('Instance') ? (route.params.id as string) : null
})

// Человекочитаемое имя открытой инстанции — источник и для шапки чата
// (постоянный индикатор), и для системного сообщения агенту (см. ниже).
// Раньше подставлялось текстом в САМО сообщение пользователя и только в
// первый ход диалога — терялось, если зайти в чат с главного экрана и
// только потом открыть инстанцию. Теперь актуализируется на каждый переход.
const currentInstanceLabel = ref<string | null>(null)
watch(
	currentInstanceId,
	async (id) => {
		if (!id) {
			currentInstanceLabel.value = null
			return
		}
		try {
			const instance = await getInstance(id)
			currentInstanceLabel.value = instance?.name ?? id
		} catch {
			currentInstanceLabel.value = id
		}
	},
	{ immediate: true },
)

// Короткие пояснения к моделям Groq — сам API отдаёт только id, без описания,
// поэтому список знаний ведём тут вручную и обновляем по мере смены линейки.
const MODEL_DESCRIPTIONS: Record<string, string> = {
	'openai/gpt-oss-120b': 'Самая толковая из бесплатных, но быстрее сажает общий бесплатный пул ключей.',
	'openai/gpt-oss-20b': 'По умолчанию. Компактнее и быстрее 120b, бережнее к общему лимиту — ответы попроще.',
	'groq/compound': 'Умеет искать в интернете и выполнять код, но не умеет пользоваться инструментами лаунчера.',
	'groq/compound-mini': 'Облегчённая версия compound — та же оговорка про инструменты лаунчера.',
	'allam-2-7b': 'Специализация на арабском языке — для агента на русском не подходит.',
}

function describeModel(id: string): string {
	return MODEL_DESCRIPTIONS[id] ?? 'Описание для этой модели пока не добавлено.'
}

// Человекочитаемые названия инструментов для свёрнутого пузыря — тех же,
// что зарегистрированы в packages/app-lib/src/api/modlex_ai.rs (см. tool_tier
// там же за актуальным списком и тирами — тут дублировать не нужно).
const TOOL_LABELS: Record<string, string> = {
	list_instances: 'Список инстанций',
	get_instance_details: 'Детали инстанции',
	list_installed_content: 'Установленный контент',
	get_last_crash_report: 'Краш-лог',
	get_launcher_log: 'Лог лаунчера',
	search_mods: 'Поиск модов',
	get_project_versions: 'Версии проекта',
	get_project_details: 'Детали проекта',
	list_java_installations: 'Установки Java',
	get_loader_versions: 'Версии загрузчика',
	save_note: 'Заметка на будущее',
	list_notes: 'Прошлые заметки',
	get_system_specs: 'Характеристики ПК',
	launch_instance: 'Запуск игры',
	create_instance: 'Создать инстанцию',
	rename_instance: 'Переименовать инстанцию',
	update_instance_content: 'Обновить моды инстанции',
	report_bug: 'Отправить баг-репорт',
	install_mod_version: 'Установить мод',
	install_mod_versions: 'Установить моды',
	remove_mod: 'Удалить мод',
	toggle_mod: 'Включить/выключить мод',
	change_mod_version: 'Сменить версию мода',
	set_instance_java: 'Назначить Java',
	set_instance_memory: 'Назначить память',
	install_java: 'Установить Java',
	set_instance_loader: 'Сменить загрузчик инстанции',
}

function toolLabel(name: string): string {
	// Модель иногда присылает вызов с пустым/битым именем (см. память проекта:
	// это уже наблюдалось и у Groq — мусор от harmony-формата, и у других
	// провайдеров как совсем пустая строка) — без фолбэка это была бы
	// полностью пустая, неотличимая от бага строка в UI. execute_tool() и
	// так сам корректно отвечает "неизвестный инструмент" в этом случае,
	// тут только про то, что видно в заголовке пузыря.
	return TOOL_LABELS[name] || name || '(нераспознанный вызов)'
}

function formatToolArguments(raw: string): string {
	try {
		return JSON.stringify(JSON.parse(raw), null, 2)
	} catch {
		return raw
	}
}

/** Tauri's invoke() rejects with whatever the Rust command's Err serializes
 * to — usually a plain string, NOT a JS Error instance (same convention as
 * `error.message ?? error` used elsewhere in this codebase, e.g.
 * web-notifications.ts's handleError) — so `e instanceof Error` alone
 * silently swallows the real backend message and falls back to a generic
 * string. */
function formatInvokeError(e: unknown): string {
	if (e instanceof Error) return e.message
	if (typeof e === 'string') return e
	return JSON.stringify(e)
}

/** Человекочитаемое описание для карточки подтверждения тир-Confirm вызова. */
function describePendingAction(name: string, argsJson: string): string {
	let args: Record<string, unknown> = {}
	try {
		args = JSON.parse(argsJson)
	} catch {
		return `Выполнить «${toolLabel(name)}» с аргументами: ${argsJson}`
	}

	switch (name) {
		case 'install_mod_version':
			return `Установить мод (id: ${args.project_id}, версия ${args.version_id}) в инстанцию ${args.instance_id} — вместе с зависимостями, если они нужны.`
		case 'install_mod_versions': {
			const mods = Array.isArray(args.mods) ? (args.mods as Array<{ project_id?: string }>) : []
			const count = mods.length
			const modWord = count === 1 ? 'мод' : count < 5 ? 'мода' : 'модов'
			return `Установить ${count} ${modWord} в инстанцию ${args.instance_id} — вместе с зависимостями, где нужно.`
		}
		case 'remove_mod':
			return `Удалить ${args.content_path} из инстанции ${args.instance_id}.`
		case 'toggle_mod':
			return `${args.enabled ? 'Включить' : 'Выключить'} ${args.content_path} в инстанции ${args.instance_id}.`
		case 'change_mod_version':
			return `Сменить версию ${args.content_path} на ${args.version_id} в инстанции ${args.instance_id}.`
		case 'set_instance_java':
			return `Назначить инстанции ${args.instance_id} Java: ${args.java_path}.`
		case 'set_instance_memory':
			return `Назначить инстанции ${args.instance_id} лимит памяти ${args.max_mb} МБ.`
		case 'install_java':
			return `Скачать и установить Java ${args.java_version}.`
		case 'set_instance_loader': {
			const versionPart = args.loader_version ? ` версии ${args.loader_version}` : ''
			const gamePart = args.game_version ? `, Minecraft ${args.game_version}` : ''
			return `Сменить загрузчик инстанции ${args.instance_id} на ${args.loader}${versionPart}${gamePart}.`
		}
		case 'launch_instance':
			return `Запустить игру для инстанции ${args.instance_id}.`
		case 'get_system_specs':
			return 'Прочитать характеристики этого ПК (ОС, процессор, объём памяти).'
		case 'create_instance':
			return `Создать инстанцию «${args.name}» (${args.game_version}, ${args.loader}).`
		case 'rename_instance':
			return `Переименовать инстанцию ${args.instance_id} в «${args.name}».`
		case 'update_instance_content':
			return `Обновить все устаревшие моды в инстанции ${args.instance_id}.`
		case 'report_bug':
			return `Отправить баг-репорт разработчику: «${args.title}».`
		default:
			return `Выполнить «${toolLabel(name)}».`
	}
}

interface TextItem {
	kind: 'text'
	/** Синтетический ключ ("text-{индекс в history}") — раньше тут был
	 * undefined для ВСЕХ текстовых пузырей разом, что ломало :key в v-for
	 * (Vue путал/переиспользовал DOM-узлы между сообщениями) и, судя по
	 * всему, было причиной "пустых плиток без реакции на клик" в чате. */
	id: string
	role: 'user' | 'assistant'
	content: string
}

// Некоторые "think"-модели (глючит не всегда, но реально наблюдалось у z.ai
// glm-4.5-flash) иногда кладут рассуждение прямо в content блоком
// <think>...</think>, а не в отдельное поле reasoning_content — тогда оно
// показывалось прямо посреди ответа пользователю. Вырезаем на отображении;
// в history, которая уходит обратно в API, содержимое остаётся как есть.
const THINK_TAG_RE = /<think>[\s\S]*?<\/think>/gi

interface ToolItem {
	kind: 'tool'
	id: string
	name: string
	arguments: string
	result: string | null
}

type RenderItem = TextItem | ToolItem

const isOpen = ref(false)
const showSettings = ref(false)

const history = ref<AiChatMessage[]>([])
const input = ref('')
const sending = ref(false)
const chatError = ref('')
// Сообщения, стримящиеся по ходу выполнения (см. modlex-ai-hop ниже) — без
// этого при длинной цепочке шагов (особенно с auto_confirm, где нет пауз на
// подтверждение между действиями) фронт не показывает НИЧЕГО, пока весь ход
// не закончится целиком, и весь текст с пузырями инструментов "вываливается"
// одним пакетом в конце. renderItems ниже склеивает history + liveMessages
// в одну последовательность; когда ход завершается, applyOutcome заменяет
// history на полный outcome.messages (уже включающий всё стримленное) и
// liveMessages чистится — визуально пузыри просто "остаются на месте".
const liveMessages = ref<AiChatMessage[]>([])
let unlistenHop: UnlistenFn | undefined
// Какое действие упало последним — чтобы кнопка "Повторить" знала, что именно
// переотправлять (сообщение не теряется/не дублируется: send() уже положил
// его в history до сетевого вызова, resume просто повторяет тот же call с
// тем же approved).
const lastFailedKind = ref<'send' | 'resume' | null>(null)
const lastResumeApproved = ref(true)
const expandedTools = ref<Set<string>>(new Set())
// Вызов тир-Confirm, ожидающий явного решения пользователя — пока он не
// null, ввод текста заблокирован (сначала разберись с этим действием).
const pendingCall = ref<AiToolCall | null>(null)

const scrollEl = ref<HTMLElement | null>(null)
const inputEl = ref<HTMLTextAreaElement | null>(null)

const models = ref<string[]>([])
const modelsLoading = ref(false)
const modelsError = ref('')

const selectedModel = ref('openai/gpt-oss-20b')
const ownApiKey = ref('')

// MODLEX: диагностика ключей ИИ-агента.
// Пинг СВОЕГО ключа — доступен всем, тратит только личную квоту пользователя.
const ownKeyPinging = ref(false)
const ownKeyPingResult = ref<KeyPingResult | null>(null)

// Пинг ВСЕГО общего пула ключей — только для сборки разработчика. devMode
// сам по себе НЕ гарантия (это просто флаг в настройках, его может включить
// кто угодно, кто найдёт секретную фразу) — devPingAvailable проверяет
// РЕАЛЬНЫЙ флаг компиляции на бэкенде (см. modlex_ai.rs), который в
// официальных сборках просто отсутствует физически.
const devPingAvailable = ref(false)
const poolPinging = ref(false)
const poolPingResults = ref<KeyPingResult[] | null>(null)
const showPoolPingPanel = ref(false)

modlex_ai_dev_ping_available()
	.then((available) => {
		devPingAvailable.value = available
	})
	.catch(() => {
		devPingAvailable.value = false
	})

async function pingOwnKey() {
	if (!ownApiKey.value.trim() || ownKeyPinging.value) return
	ownKeyPinging.value = true
	ownKeyPingResult.value = null
	try {
		ownKeyPingResult.value = await modlex_ai_ping_own_key(ownApiKey.value.trim())
	} catch (e) {
		ownKeyPingResult.value = {
			provider: 'z.ai',
			keyPreview: '',
			status: 'error',
			latencyMs: 0,
			detail: e instanceof Error ? e.message : String(e),
			quotaHint: null,
		}
	} finally {
		ownKeyPinging.value = false
	}
}

const PING_STATUS_LABELS: Record<KeyPingResult['status'], string> = {
	ok: 'Работает',
	rateLimited: 'Лимит исчерпан',
	invalidKey: 'Ключ невалиден',
	error: 'Ошибка',
}
function pingStatusLabel(status: KeyPingResult['status']): string {
	return PING_STATUS_LABELS[status] ?? status
}

async function pingKeyPool() {
	if (poolPinging.value) return
	poolPinging.value = true
	showPoolPingPanel.value = true
	try {
		poolPingResults.value = await modlex_ai_ping_key_pool()
	} catch (e) {
		poolPingResults.value = []
		console.error('Не удалось проверить пул ключей:', e)
	} finally {
		poolPinging.value = false
	}
}

// Заглушка в списке провайдеров — своя нейросеть под ModLEX/Minecraft, обучение
// на своём железе, сроков нет вообще (не "скоро" — оттого "???", а не бейдж
// в духе "СКОРО"). Невыбираема нарочно: реального такого провайдера ещё не
// существует, вызов с ним просто упал бы на бэкенде.
const MODLEX_AI_PLACEHOLDER = 'modlex-ai'

const PROVIDER_OPTIONS = ['auto', 'groq', 'zai', 'cloudflare', MODLEX_AI_PLACEHOLDER] as const
const PROVIDER_LABELS: Record<string, string> = {
	auto: 'Авто (по очереди: z.ai → Cloudflare → Groq)',
	groq: 'Только Groq (не рекомендуется — на крайний случай)',
	zai: 'Только z.ai',
	cloudflare: 'Только Cloudflare',
	[MODLEX_AI_PLACEHOLDER]: 'ModLEX AI [???]',
}
function providerLabel(id: string): string {
	return PROVIDER_LABELS[id] ?? id
}
const selectedProvider = ref('auto')

const autoConfirm = ref(false)
const autoConfirmModal = ref<InstanceType<typeof ConfirmModal>>()

// 0 = дефолт бэкенда (см. modlex_ai::resolve_max_hops). Комплексные задачи
// (заменить сразу 10 модов) могут упереться в маленький лимит и оборваться,
// не закончив дело — настраиваемо, чтобы не приходилось лезть в код.
const maxHops = ref(0)

// requestSeq — стоп-кнопка не умеет реально прервать уже улетевший запрос
// (в этой версии Tauri у invoke() нет AbortSignal), поэтому просто помечает
// текущий запрос как "устаревший": когда он всё же ответит, результат тихо
// игнорируется. Для read-only тир-0 инструментов это безопасно — прервать
// сам процесс на бэке понадобится только когда появятся тир-1 действия.
let requestSeq = 0

const renderItems = computed<RenderItem[]>(() => {
	const items: RenderItem[] = []
	const toolIndexById = new Map<string, number>()
	const allMessages = [...history.value, ...liveMessages.value]

	for (const [historyIndex, message] of allMessages.entries()) {
		if (message.role === 'system') continue

		if (message.role === 'tool') {
			const idx = message.tool_call_id ? toolIndexById.get(message.tool_call_id) : undefined
			if (idx !== undefined) {
				const item = items[idx]
				if (item.kind === 'tool') item.result = message.content ?? ''
			}
			continue
		}

		// z.ai (в отличие от Groq) присылает content: "\n" даже когда ассистент
		// ТОЛЬКО вызывает инструмент, а не null/пусто — просто .trim() тут
		// недостаточно как проверка "истинности": "\n" истинно в JS, поэтому
		// без .trim() получался пустой текстовый пузырь без единого видимого
		// символа (и, что важнее, вообще без обработчика клика — это не
		// пузырь инструмента, у него нет @click вовсе).
		const visibleContent = (message.content ?? '').replace(THINK_TAG_RE, '').trim()
		if (visibleContent) {
			items.push({
				kind: 'text',
				id: `text-${historyIndex}`,
				role: message.role as 'user' | 'assistant',
				content: visibleContent,
			})
		}
		if (message.tool_calls) {
			for (const call of message.tool_calls) {
				toolIndexById.set(call.id, items.length)
				items.push({
					kind: 'tool',
					id: call.id,
					name: call.function.name,
					arguments: call.function.arguments,
					result: null,
				})
			}
		}
	}

	return items
})

function toggleToolExpanded(id: string) {
	const next = new Set(expandedTools.value)
	if (next.has(id)) {
		next.delete(id)
	} else {
		next.add(id)
	}
	expandedTools.value = next
}

async function scrollToBottom() {
	await nextTick()
	if (scrollEl.value) {
		scrollEl.value.scrollTop = scrollEl.value.scrollHeight
	}
}

async function loadSettingsIntoWidget() {
	const settings = await getSettings()
	selectedModel.value = settings.modlex_ai_model
	selectedProvider.value = settings.modlex_ai_provider || 'auto'
	ownApiKey.value = settings.modlex_ai_api_key ?? ''
	autoConfirm.value = settings.modlex_ai_auto_confirm ?? false
	maxHops.value = settings.modlex_ai_max_hops ?? 0
}

let maxHopsDebounce: ReturnType<typeof setTimeout> | undefined
function onMaxHopsInput(value: string | number) {
	const parsed = Math.max(0, Math.floor(Number(value) || 0))
	maxHops.value = parsed
	clearTimeout(maxHopsDebounce)
	maxHopsDebounce = setTimeout(async () => {
		const settings = await getSettings()
		settings.modlex_ai_max_hops = parsed
		await setSettings(settings)
	}, 400)
}

async function persistModel(model: string) {
	const settings = await getSettings()
	settings.modlex_ai_model = model
	await setSettings(settings)
}

// DropdownSelect сам не умеет отклонять клик по опции — если просто
// проигнорировать смену в onProviderChange, компонент всё равно визуально
// "выберет" заглушку внутри себя (его internal selectedValue не откатывается
// сам, раз bound props.modelValue не поменялся). Форсируем ремонт через :key,
// чтобы он заново подхватил реальный selectedProvider.
const providerDropdownKey = ref(0)

async function onProviderChange(provider: string) {
	// Заглушка в списке — реального провайдера пока нет, выбрать нельзя.
	if (provider === MODLEX_AI_PLACEHOLDER) {
		providerDropdownKey.value++
		return
	}
	selectedProvider.value = provider
	const settings = await getSettings()
	settings.modlex_ai_provider = provider
	await setSettings(settings)
}

/** Включение требует одноразового подтверждения через ConfirmModal (см.
 * шаблон) — выключение обратно не предупреждает, это только сужение риска. */
async function persistAutoConfirm(value: boolean) {
	autoConfirm.value = value
	const settings = await getSettings()
	settings.modlex_ai_auto_confirm = value
	await setSettings(settings)
}

function onAutoConfirmToggle(value: boolean) {
	if (value) {
		autoConfirmModal.value?.show()
	} else {
		persistAutoConfirm(false)
	}
}

function confirmAutoConfirmEnable() {
	persistAutoConfirm(true)
}

let ownKeyDebounce: ReturnType<typeof setTimeout> | undefined
function onOwnKeyInput(value: string) {
	ownApiKey.value = value
	clearTimeout(ownKeyDebounce)
	ownKeyDebounce = setTimeout(async () => {
		const settings = await getSettings()
		settings.modlex_ai_api_key = value.length > 0 ? value : null
		await setSettings(settings)
	}, 600)
}

async function onModelChange(model: string) {
	selectedModel.value = model
	await persistModel(model)
}

async function ensureModelsLoaded() {
	if (models.value.length > 0 || modelsLoading.value) return
	modelsLoading.value = true
	modelsError.value = ''
	try {
		const list = await modlex_ai_list_models()
		models.value = list.map((m) => m.id)
	} catch (e) {
		modelsError.value = formatInvokeError(e)
	} finally {
		modelsLoading.value = false
	}
}

onMounted(async () => {
	unlistenHop = await listen<AiChatMessage>('modlex-ai-hop', (event) => {
		// Стоп-кнопка не умеет реально прервать уже улетевший бэкенд-вызов
		// (см. requestSeq выше) — если ход уже остановлен/завершён, а
		// события всё ещё долетают, не показываем их: иначе после Stop
		// могли бы неожиданно появляться новые пузыри.
		if (!sending.value) return
		liveMessages.value = [...liveMessages.value, event.payload]
		scrollToBottom()
	})
})

onBeforeUnmount(() => {
	unlistenHop?.()
})

/** Открывает панель и сразу отправляет сообщение — вызывается извне (кнопки
 * "Спросить ИИ" на тостах ошибок и т.п.) через pendingAiAgentHelpRequest, см.
 * composables/use-ai-agent-bridge.ts. */
watch(pendingAiAgentHelpRequest, async (request) => {
	if (!request) return
	pendingAiAgentHelpRequest.value = null

	if (!isOpen.value) {
		isOpen.value = true
		await loadSettingsIntoWidget()
	}
	input.value = request.message
	await send()
})

async function toggleOpen() {
	isOpen.value = !isOpen.value
	if (isOpen.value) {
		await loadSettingsIntoWidget()
		await scrollToBottom()
		await nextTick()
		inputEl.value?.focus()
	} else {
		// Иначе при следующем открытии чат появлялся со свёрнутыми в него
		// настройками с прошлого раза (showSettings переживает закрытие
		// панели, т.к. это отдельный ref, а не часть isOpen) — тесня
		// пустое состояние/сообщения в оставшееся место.
		showSettings.value = false
	}
}

function toggleSettings() {
	showSettings.value = !showSettings.value
	if (showSettings.value) {
		ensureModelsLoaded()
	}
}

function stopAgent() {
	requestSeq++ // делает любой ещё летящий ответ устаревшим — будет проигнорирован
	sending.value = false
}

/** Если ход упал (например, сеть моргнула на середине долгого поиска с кучей
 * хопов), сообщения, уже прилетевшие через modlex-ai-hop (см. liveMessages),
 * иначе просто исчезают — backend ничего не сохраняет (сессии на сервере
 * нет), а finally ниже стирает liveMessages безусловно. Пользователь в этом
 * случае терял ВСЮ проделанную работу (поиск модов, решения) и должен был
 * начинать с нуля. Переносим то, что реально успело прийти, в history — это
 * НЕ история именно этого("успешного") хода, а честный хвост того, что
 * агент уже сделал до обрыва; следующий "Повторить" уйдёт уже с этим
 * контекстом, а не с чистого листа. */
function salvageLiveMessages() {
	if (liveMessages.value.length === 0) return
	history.value = [...history.value, ...liveMessages.value]
}

/** Общая обработка результата хода — используется и send(), и confirmPending(),
 * т.к. возобновлённый ход тоже может тут же упереться в СЛЕДУЮЩЕЕ подтверждение. */
function applyOutcome(outcome: AgentStepOutcome) {
	history.value = outcome.messages
	if (outcome.outcome === 'pending_confirmation') {
		pendingCall.value = outcome.pending_call
		// Разворачиваем пузырь сразу — пользователю нужно видеть аргументы,
		// чтобы решить, подтверждать действие или нет, без лишнего клика.
		expandedTools.value = new Set(expandedTools.value).add(outcome.pending_call.id)
	} else {
		pendingCall.value = null
	}
}

/** Отмечает системное сообщение с контекстом открытой инстанции, чтобы можно
 * было найти и обновить именно его (а не текст пользователя, как раньше). */
const CONTEXT_SYSTEM_MARKER = '[modlex-instance-context]'

/** Актуализирует контекст открытой инстанции перед каждым ходом — не только
 * первым. Раньше это было текстом внутри первого сообщения пользователя и
 * навсегда застревало таким, каким было при старте диалога: если зайти в
 * чат с главного экрана, а потом открыть инстанцию (или переключиться на
 * другую), агент об этом так и не узнавал. Системное сообщение не показывается
 * пользователю (renderItems пропускает role: 'system'), только модели. */
function syncContextMessage() {
	const others = history.value.filter(
		(m) => !(m.role === 'system' && m.content?.startsWith(CONTEXT_SYSTEM_MARKER)),
	)
	if (!currentInstanceId.value) {
		history.value = others
		return
	}
	const label = currentInstanceLabel.value ?? currentInstanceId.value
	history.value = [
		{
			role: 'system',
			content: `${CONTEXT_SYSTEM_MARKER} Пользователь сейчас смотрит на инстанцию "${label}" (id: ${currentInstanceId.value}). Если он явно не называет другую инстанцию, речь о ней.`,
		},
		...others,
	]
}

/** Собственно сетевой ход — вынесен отдельно от send(), чтобы "Повторить"
 * могло переотправить тот же запрос, не дублируя уже добавленное в history
 * сообщение пользователя. */
async function runAgentStep() {
	sending.value = true
	chatError.value = ''
	liveMessages.value = []
	await scrollToBottom()

	const myRequestSeq = ++requestSeq
	try {
		const outcome = await modlex_ai_agent_step(history.value)
		if (myRequestSeq !== requestSeq) return // остановлено или обогнано новым запросом
		applyOutcome(outcome)
		lastFailedKind.value = null
	} catch (e) {
		if (myRequestSeq !== requestSeq) return
		salvageLiveMessages()
		chatError.value = formatInvokeError(e)
		lastFailedKind.value = 'send'
	} finally {
		if (myRequestSeq === requestSeq) {
			sending.value = false
			liveMessages.value = []
		}
		await scrollToBottom()
	}
}

async function send() {
	const text = input.value.trim()
	if (!text || sending.value || pendingCall.value) return

	syncContextMessage()
	history.value.push({ role: 'user', content: text })
	input.value = ''
	await runAgentStep()
}

/** Пользователь нажал "Подтвердить" или "Отклонить" в карточке действия. */
async function confirmPending(approved: boolean) {
	const call = pendingCall.value
	if (!call || sending.value) return

	lastResumeApproved.value = approved
	sending.value = true
	chatError.value = ''
	liveMessages.value = []
	await scrollToBottom()

	const myRequestSeq = ++requestSeq
	try {
		const outcome = await modlex_ai_agent_resume(history.value, call, approved)
		if (myRequestSeq !== requestSeq) return
		applyOutcome(outcome)
		lastFailedKind.value = null
	} catch (e) {
		if (myRequestSeq !== requestSeq) return
		salvageLiveMessages()
		chatError.value = formatInvokeError(e)
		lastFailedKind.value = 'resume'
	} finally {
		if (myRequestSeq === requestSeq) {
			sending.value = false
			liveMessages.value = []
		}
		await scrollToBottom()
	}
}

/** Кнопка "Повторить" под сообщением/ошибкой — переотправляет ровно тот же
 * запрос, который упал (сетевая ошибка чаще всего временная). */
async function retryLastAction() {
	if (sending.value) return
	if (lastFailedKind.value === 'send') {
		await runAgentStep()
	} else if (lastFailedKind.value === 'resume') {
		await confirmPending(lastResumeApproved.value)
	}
}

function onInputKeydown(event: KeyboardEvent) {
	if (event.key === 'Enter' && !event.shiftKey) {
		event.preventDefault()
		send()
	}
}
</script>

<template>
	<div class="modlex-ai-agent">
		<!-- Один общий контур лапы для "думаю..." — переиспользуется через <use>,
		     чтобы не дублировать координаты в каждом месте. -->
		<svg width="0" height="0" style="position: absolute" aria-hidden="true">
			<symbol id="modlex-ai-paw" viewBox="0 0 24 24">
				<ellipse cx="12" cy="16.5" rx="6.2" ry="4.6" />
				<circle cx="5.6" cy="7.4" r="2.5" />
				<circle cx="10.2" cy="4.4" r="2.7" />
				<circle cx="15.4" cy="4.4" r="2.7" />
				<circle cx="19.6" cy="7.6" r="2.4" />
			</symbol>
		</svg>
		<Transition name="ai-panel">
			<div v-if="isOpen" class="ai-panel">
				<div class="ai-panel__header">
					<BotIcon class="ai-panel__header-icon" />
					<div class="ai-panel__header-titles">
						<span class="ai-panel__title">ИИ-агент ModLEX</span>
						<span v-if="currentInstanceLabel" class="ai-panel__header-instance">
							📍 {{ currentInstanceLabel }}
						</span>
					</div>
					<IconButton
						v-tooltip="'Модель и настройки'"
						label="Модель и настройки"
						native-type="button"
						:class="{ 'ai-panel__settings-btn--active': showSettings }"
						@click="toggleSettings"
					>
						<SettingsIcon />
					</IconButton>
					<IconButton
						v-tooltip="'Закрыть'"
						label="Закрыть"
						native-type="button"
						@click="toggleOpen"
					>
						<XIcon />
					</IconButton>
				</div>

				<div v-if="showSettings" class="ai-panel__settings">
					<div class="ai-panel__settings-row">
						<label class="ai-panel__settings-label">Провайдер</label>
						<DropdownSelect
							:key="providerDropdownKey"
							name="modlex-ai-provider"
							:options="[...PROVIDER_OPTIONS]"
							:display-name="providerLabel"
							:model-value="selectedProvider"
							@update:model-value="onProviderChange"
						/>
					</div>
					<p class="ai-panel__settings-desc">
						"Авто" перебирает провайдеров по очереди при исчерпании лимита. Выбери конкретного,
						чтобы протестировать его в изоляции.
					</p>

					<div class="ai-panel__settings-row">
						<label class="ai-panel__settings-label">Модель</label>
						<DropdownSelect
							name="modlex-ai-model"
							:options="models"
							:model-value="selectedModel"
							:placeholder="modelsLoading ? 'Загрузка списка...' : selectedModel"
							@update:model-value="onModelChange"
						/>
					</div>
					<p v-if="modelsError" class="ai-panel__settings-error">{{ modelsError }}</p>
					<p class="ai-panel__settings-desc">{{ describeModel(selectedModel) }}</p>

					<div class="ai-panel__settings-row">
						<label class="ai-panel__settings-label" for="modlex-ai-own-key">
							Свой ключ z.ai (опционально)
						</label>
						<StyledInput
							id="modlex-ai-own-key"
							type="password"
							placeholder="Если общие ключи исчерпали лимит"
							:model-value="ownApiKey"
							@update:model-value="onOwnKeyInput"
						/>
					</div>
					<div class="ai-panel__settings-row ai-panel__settings-row--inline">
						<Button
							type="transparent"
							native-type="button"
							:disabled="!ownApiKey.trim() || ownKeyPinging"
							@click="pingOwnKey"
						>
							<KeyIcon />
							{{ ownKeyPinging ? 'Проверяю…' : 'Пингануть свой ключ' }}
						</Button>
					</div>
					<div v-if="ownKeyPingResult" class="ai-panel__key-ping-row">
						<KeyIcon class="ai-panel__key-ping-icon" />
						<span
							class="ai-panel__key-ping-status"
							:class="`ai-panel__key-ping-status--${ownKeyPingResult.status}`"
						>
							{{ pingStatusLabel(ownKeyPingResult.status) }} · {{ ownKeyPingResult.latencyMs }} мс
							<template v-if="ownKeyPingResult.detail">— {{ ownKeyPingResult.detail }}</template>
						</span>
						<span class="ai-panel__key-ping-preview">{{ ownKeyPingResult.keyPreview || '—' }}</span>
					</div>
					<p v-if="ownKeyPingResult?.quotaHint" class="ai-panel__settings-desc">
						{{ ownKeyPingResult.quotaHint }}
					</p>

					<div class="ai-panel__settings-row">
						<label class="ai-panel__settings-label" for="modlex-ai-auto-confirm">
							Не спрашивать подтверждения
						</label>
						<Toggle
							id="modlex-ai-auto-confirm"
							:model-value="autoConfirm"
							@update:model-value="onAutoConfirmToggle"
						/>
					</div>
					<p class="ai-panel__settings-desc">
						Установка, удаление и смена версий модов будут выполняться сразу, без карточки
						подтверждения. Не влияет на действия, которые требуют доступа к системным данным —
						их всегда нужно подтверждать отдельно.
					</p>

					<div class="ai-panel__settings-row">
						<label class="ai-panel__settings-label" for="modlex-ai-max-hops">
							Лимит шагов за один ход
						</label>
						<StyledInput
							id="modlex-ai-max-hops"
							type="number"
							min="0"
							placeholder="По умолчанию"
							:model-value="maxHops || ''"
							@update:model-value="onMaxHopsInput"
						/>
					</div>
					<p class="ai-panel__settings-desc">
						Сколько действий подряд агент может выполнить сам, прежде чем ответить или
						остановиться с "не смог разобраться". 0 — использовать значение по умолчанию.
						Комплексные задачи (заменить сразу много модов) могут требовать большего лимита.
					</p>

					<template v-if="appSettings.devMode && devPingAvailable">
						<div class="ai-panel__settings-row ai-panel__settings-row--inline">
							<Button
								type="transparent"
								native-type="button"
								:disabled="poolPinging"
								@click="pingKeyPool"
							>
								<KeyIcon />
								{{ poolPinging ? 'Проверяю пул…' : 'Проверить весь пул ключей' }}
							</Button>
						</div>
						<p class="ai-panel__settings-desc">
							Только для этой сборки — проверяет каждый встроенный ключ по всем провайдерам.
						</p>

						<div v-if="showPoolPingPanel" class="ai-panel__pool-ping">
							<div class="ai-panel__pool-ping-header">
								<span>Проверка пула ключей</span>
								<IconButton @click="showPoolPingPanel = false">
									<XIcon />
								</IconButton>
							</div>
							<p v-if="poolPinging" class="ai-panel__settings-desc">
								Пингую все ключи, это может занять некоторое время…
							</p>
							<div v-else-if="poolPingResults" class="ai-panel__pool-ping-list">
								<p v-if="poolPingResults.length === 0" class="ai-panel__settings-desc">
									Пул пуст.
								</p>
								<div
									v-for="(result, i) in poolPingResults"
									:key="i"
									class="ai-panel__key-ping-row"
								>
									<KeyIcon class="ai-panel__key-ping-icon" />
									<span class="ai-panel__key-ping-provider">{{ result.provider }}</span>
									<span
										class="ai-panel__key-ping-status"
										:class="`ai-panel__key-ping-status--${result.status}`"
									>
										{{ pingStatusLabel(result.status) }} · {{ result.latencyMs }} мс
										<template v-if="result.detail">— {{ result.detail }}</template>
									</span>
									<span class="ai-panel__key-ping-preview">{{ result.keyPreview }}</span>
								</div>
							</div>
						</div>
					</template>
				</div>

				<ConfirmModal
					ref="autoConfirmModal"
					title="Отключить подтверждение действий?"
					description="ИИ-агент сможет сразу устанавливать, удалять и менять версии модов — без запроса подтверждения на каждый шаг. В любой момент можно выключить обратно в настройках."
					proceed-label="Включить"
					danger
					@proceed="confirmAutoConfirmEnable"
				/>

				<template v-if="!showSettings">
				<div ref="scrollEl" class="ai-panel__messages">
					<div v-if="renderItems.length === 0" class="ai-panel__empty">
						<BotIcon class="ai-panel__empty-icon" />
						<p class="ai-panel__empty-text">
							Спроси про ошибку, моды, сборки или настройку лаунчера — помогу разобраться.
						</p>
					</div>

					<template v-for="item in renderItems" :key="item.id">
						<div
							v-if="item.kind === 'text'"
							class="ai-message"
							:class="`ai-message--${item.role}`"
						>
							{{ item.content }}
						</div>

						<div
							v-else
							class="ai-tool-call"
							:class="{ 'ai-tool-call--pending': pendingCall?.id === item.id }"
						>
							<button
								type="button"
								class="ai-tool-call__header"
								@click="toggleToolExpanded(item.id)"
							>
								<WrenchIcon class="ai-tool-call__icon" />
								<span class="ai-tool-call__label">{{ toolLabel(item.name) }}</span>
								<svg
									v-if="item.result === null && pendingCall?.id !== item.id"
									class="ai-paw ai-paw--sm"
									viewBox="0 0 24 24"
									aria-hidden="true"
								><use href="#modlex-ai-paw" /></svg>
								<component
									:is="expandedTools.has(item.id) ? ChevronDownIcon : ChevronRightIcon"
									class="ai-tool-call__chevron"
								/>
							</button>
							<div v-if="expandedTools.has(item.id)" class="ai-tool-call__details">
								<div class="ai-tool-call__section">
									<span class="ai-tool-call__section-title">Аргументы</span>
									<pre>{{ formatToolArguments(item.arguments) }}</pre>
								</div>
								<div v-if="item.result !== null" class="ai-tool-call__section">
									<span class="ai-tool-call__section-title">Результат</span>
									<pre>{{ item.result }}</pre>
								</div>
							</div>
							<!-- Вне тумблера "развернуть" — подтверждение должно быть видно
							     и доступно кликом сразу, без необходимости разворачивать
							     карточку ради двух кнопок. -->
							<div v-if="pendingCall?.id === item.id" class="ai-tool-call__confirm">
								<p class="ai-tool-call__confirm-text">
									{{ describePendingAction(item.name, item.arguments) }}
								</p>
								<div class="ai-tool-call__confirm-actions">
									<Button
										type="colored"
										color="red"
										native-type="button"
										:disabled="sending"
										@click="confirmPending(false)"
									>
										<XIcon />
										Отклонить
									</Button>
									<Button
										type="colored"
										color="brand"
										native-type="button"
										:disabled="sending"
										@click="confirmPending(true)"
									>
										<CheckIcon />
										Подтвердить
									</Button>
								</div>
							</div>
						</div>
					</template>

					<div v-if="sending" class="ai-message ai-message--assistant ai-message--pending">
						<span class="ai-paw-trail" aria-hidden="true">
							<svg class="ai-paw" viewBox="0 0 24 24"><use href="#modlex-ai-paw" /></svg>
							<svg class="ai-paw" viewBox="0 0 24 24"><use href="#modlex-ai-paw" /></svg>
							<svg class="ai-paw" viewBox="0 0 24 24"><use href="#modlex-ai-paw" /></svg>
						</span>
					</div>
				</div>

				<div v-if="chatError" class="ai-panel__chat-error-row">
					<p class="ai-panel__chat-error">{{ chatError }}</p>
					<IconButton
						v-if="lastFailedKind"
						v-tooltip="'Повторить попытку'"
						label="Повторить попытку"
						native-type="button"
						:disabled="sending"
						@click="retryLastAction"
					>
						<RefreshCwIcon />
					</IconButton>
				</div>

				<div class="ai-panel__input-row">
					<textarea
						ref="inputEl"
						v-model="input"
						class="ai-panel__textarea"
						rows="1"
						:placeholder="pendingCall ? 'Сначала разберись с действием выше...' : 'Написать сообщение...'"
						:disabled="sending || !!pendingCall"
						@keydown="onInputKeydown"
					/>
					<IconButton
						v-if="sending"
						v-tooltip="'Остановить'"
						label="Остановить"
						native-type="button"
						color="red"
						@click="stopAgent"
					>
						<StopCircleIcon />
					</IconButton>
					<Button
						v-else
						type="colored"
						color="brand"
						native-type="button"
						:disabled="!input.trim() || !!pendingCall"
						@click="send"
					>
						<SendIcon />
					</Button>
				</div>
				</template>
			</div>
		</Transition>

		<button
			type="button"
			class="ai-toggle-btn"
			:class="{ 'ai-toggle-btn--open': isOpen }"
			:aria-label="isOpen ? 'Закрыть ИИ-агента' : 'Открыть ИИ-агента'"
			@click="toggleOpen"
		>
			<XIcon v-if="isOpen" />
			<BotIcon v-else />
		</button>
	</div>
</template>

<style scoped>
.modlex-ai-agent {
	position: fixed;
	right: 1rem;
	bottom: 1rem;
	z-index: 50;
	display: flex;
	flex-direction: column;
	align-items: flex-end;
	gap: 0.75rem;
}

.ai-toggle-btn {
	display: flex;
	align-items: center;
	justify-content: center;
	width: 3rem;
	height: 3rem;
	border-radius: 50%;
	border: none;
	background: var(--color-brand);
	color: var(--color-brand-inverted, #fff);
	cursor: pointer;
	box-shadow: 0 4px 16px rgba(0, 0, 0, 0.35);
	transition:
		transform 0.15s ease-out,
		background 0.15s ease-out;
}

.ai-toggle-btn svg {
	width: 1.5rem;
	height: 1.5rem;
}

.ai-toggle-btn:hover {
	transform: scale(1.05);
}

.ai-toggle-btn--open {
	background: var(--color-button-bg);
	color: var(--color-text-default);
}

.ai-panel {
	display: flex;
	flex-direction: column;
	width: 340px;
	max-height: min(560px, 70vh);
	background: var(--color-raised-bg);
	border: 1px solid var(--color-divider);
	border-radius: 12px;
	box-shadow: 0 8px 32px rgba(0, 0, 0, 0.45);
	overflow: hidden;
}

.ai-panel-enter-active,
.ai-panel-leave-active {
	transition:
		opacity 0.15s ease-out,
		transform 0.15s ease-out;
}

.ai-panel-enter-from,
.ai-panel-leave-to {
	opacity: 0;
	transform: translateY(0.5rem) scale(0.98);
}

.ai-panel__header {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	padding: 0.5rem 0.5rem 0.5rem 0.75rem;
	border-bottom: 1px solid var(--color-divider);
	flex-shrink: 0;
}

.ai-panel__header-icon {
	width: 1.1rem;
	height: 1.1rem;
	color: var(--color-brand);
	flex-shrink: 0;
}

.ai-panel__title {
	font-weight: 600;
	font-size: 0.9rem;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.ai-panel__settings-btn--active {
	color: var(--color-brand);
}

.ai-panel__settings {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	padding: 0.75rem;
	border-bottom: 1px solid var(--color-divider);
	/* MODLEX: messages/input are hidden entirely while settings is open (see
	   template — `v-if="!showSettings"`), so this is the only flexible child
	   left and can freely grow to fill the panel's own max-height budget
	   (.ai-panel). Before this, messages+input stayed mounted alongside
	   settings and their min-height left almost nothing for settings to
	   flex into, no matter what max-height was set here directly. */
	flex: 1 1 auto;
	min-height: 0;
	overflow-y: auto;
}

.ai-panel__settings-row {
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
}

.ai-panel__settings-label {
	font-size: 0.75rem;
	color: var(--color-text-secondary, var(--color-text-tertiary));
}

.ai-panel__settings-desc,
.ai-panel__settings-error {
	margin: 0;
	font-size: 0.75rem;
	color: var(--color-text-tertiary);
}

.ai-panel__settings-error {
	color: var(--color-red, #e06060);
}

.ai-panel__settings-row--inline {
	flex-direction: row;
	align-items: center;
}

.ai-panel__key-ping-row {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	padding: 0.375rem 0.5rem;
	border-radius: 8px;
	background: var(--color-button-bg);
	font-size: 0.75rem;
}

.ai-panel__key-ping-icon {
	width: 0.9rem;
	height: 0.9rem;
	flex-shrink: 0;
	color: var(--color-text-tertiary);
}

.ai-panel__key-ping-provider {
	flex-shrink: 0;
	color: var(--color-text-secondary, var(--color-text-tertiary));
	font-weight: 600;
}

.ai-panel__key-ping-status {
	flex-grow: 1;
	min-width: 0;
	text-align: center;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.ai-panel__key-ping-status--ok {
	color: var(--color-green, #4caf50);
}

.ai-panel__key-ping-status--rateLimited {
	color: var(--color-orange, #d99a2b);
}

.ai-panel__key-ping-status--invalidKey,
.ai-panel__key-ping-status--error {
	color: var(--color-red, #e06060);
}

.ai-panel__key-ping-preview {
	flex-shrink: 0;
	font-family: var(--mono-font, monospace);
	color: var(--color-text-tertiary);
}

.ai-panel__pool-ping {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	padding: 0.5rem;
	border-radius: 10px;
	border: 1px solid var(--color-divider);
	background: var(--color-bg);
	max-height: 220px;
	overflow-y: auto;
	/* MODLEX: a flex child with overflow != visible gets its "automatic
	   minimum size" treated as 0 per the flexbox spec, so without this the
	   shrink algorithm was free to crush this box down to nothing to make
	   room for its (non-overflowing, so shrink-resistant) siblings — even
	   when there was slack elsewhere in the settings panel. flex-shrink: 0
	   makes it hold its max-height/content size; the outer .ai-panel__settings
	   scroll (which also has min-height: 0 for the same reason) absorbs
	   whatever doesn't fit instead. */
	flex-shrink: 0;
}

.ai-panel__pool-ping-header {
	display: flex;
	align-items: center;
	justify-content: space-between;
	font-size: 0.8rem;
	font-weight: 600;
	color: var(--color-text-secondary, var(--color-text-tertiary));
}

.ai-panel__pool-ping-list {
	display: flex;
	flex-direction: column;
	gap: 0.375rem;
}

.ai-panel__messages {
	flex-grow: 1;
	overflow-y: auto;
	padding: 0.75rem;
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	min-height: 120px;
}

.ai-panel__empty {
	margin: auto 0;
	display: flex;
	flex-direction: column;
	align-items: center;
	gap: 0.5rem;
	padding: 0 0.5rem;
}

.ai-panel__empty-icon {
	width: 2.25rem;
	height: 2.25rem;
	color: var(--color-brand);
	opacity: 0.35;
}

.ai-panel__empty-text {
	margin: 0;
	font-size: 0.8rem;
	color: var(--color-text-tertiary);
	text-align: center;
}

.ai-message {
	max-width: 85%;
	padding: 0.5rem 0.75rem;
	border-radius: 10px;
	font-size: 0.85rem;
	line-height: 1.4;
	white-space: pre-wrap;
	word-break: break-word;
}

.ai-message--user {
	align-self: flex-end;
	background: var(--color-brand);
	color: var(--color-brand-inverted, #fff);
}

.ai-panel__header-titles {
	display: flex;
	flex-direction: column;
	flex-grow: 1;
	min-width: 0;
}

.ai-panel__header-instance {
	font-size: 0.7rem;
	opacity: 0.75;
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.ai-message--assistant {
	align-self: flex-start;
	background: var(--color-button-bg);
	color: var(--color-text-default);
}

.ai-message--pending {
	display: flex;
	align-items: center;
}

/* Дорожка из следов лапы вместо обезличенного крутящегося значка — три
   иконки по очереди подпрыгивают и высветляются, как индикатор набора
   текста, но в теме лаунчера. */
.ai-paw-trail {
	display: flex;
	align-items: flex-end;
	gap: 0.2rem;
}

.ai-paw {
	width: 0.85rem;
	height: 0.85rem;
	fill: var(--color-brand);
	animation: ai-paw-bounce 1.1s ease-in-out infinite;
}

.ai-paw-trail .ai-paw:nth-child(2) {
	animation-delay: 0.15s;
}

.ai-paw-trail .ai-paw:nth-child(3) {
	animation-delay: 0.3s;
}

.ai-paw--sm {
	width: 0.7rem;
	height: 0.7rem;
	fill: var(--color-text-default);
	animation: ai-paw-fade 1.2s ease-in-out infinite;
}

@keyframes ai-paw-bounce {
	0%,
	60%,
	100% {
		opacity: 0.35;
		transform: translateY(0) scale(0.85);
	}
	30% {
		opacity: 1;
		transform: translateY(-3px) scale(1);
	}
}

@keyframes ai-paw-fade {
	0%,
	100% {
		opacity: 0.4;
		transform: scale(0.9);
	}
	50% {
		opacity: 1;
		transform: scale(1.05);
	}
}

@media (prefers-reduced-motion: reduce) {
	.ai-paw,
	.ai-paw--sm {
		animation: none;
	}
}

.ai-tool-call {
	/* Раньше была "width: fit-content" (компактный чип по содержимому, а
	   не на всю строку) — но в WebView2 (Tauri) это в паре с
	   "overflow: hidden" реально схлопывало высоту блока почти в ноль
	   (не проблема цвета/контраста — проверено отдельно), а без overflow
	   давало наложение соседних карточек друг на друга при длинном
	   содержимом. shrink-to-fit тут просто ненадёжен в этом движке —
	   поэтому теперь на всю ширину, как и всегда было у pending-варианта
	   ниже (там ни разу не было ни одного из этих багов). Компактность
	   сохраняем только по высоте — маленький padding/font в хедере. */
	width: 100%;
	border: 1px solid var(--color-divider);
	border-radius: 8px;
	background: var(--color-button-bg);
}

.ai-tool-call--pending {
	border-color: var(--color-brand);
}

.ai-tool-call__header {
	display: flex;
	align-items: center;
	gap: 0.3rem;
	width: 100%;
	padding: 0.2rem 0.5rem;
	border: none;
	background: transparent;
	/* НЕ --color-text-tertiary: в кастомной теме ModLEX это независимый
	   слайдер (shade кастомного цвета текста, коэфф. 0.6) от цвета фона
	   кнопок --color-button-bg (тоже независимый слайдер, "Панели и
	   карточки") — при некоторых пользовательских сочетаниях они почти
	   совпадают, и весь чип становится нечитаемым (см. память проекта).
	   --color-text-default — тот же токен, что и у обычных текстовых
	   пузырей чата, которые остаются читаемыми при любых кастомных цветах. */
	color: var(--color-text-default);
	font-size: 0.7rem;
	line-height: 1.3;
	cursor: pointer;
	text-align: left;
	white-space: nowrap;
}

.ai-tool-call__icon {
	width: 0.7rem;
	height: 0.7rem;
	color: var(--color-text-default);
	flex-shrink: 0;
}

.ai-tool-call__label {
	flex-grow: 1;
}

.ai-tool-call__chevron {
	width: 0.7rem;
	height: 0.7rem;
	color: var(--color-text-default);
	flex-shrink: 0;
}

.ai-tool-call--pending .ai-tool-call__header {
	padding: 0.4rem 0.6rem;
	font-size: 0.8rem;
	color: var(--color-text-default);
	white-space: normal;
}

.ai-tool-call--pending .ai-tool-call__icon,
.ai-tool-call--pending .ai-tool-call__chevron {
	width: 0.9rem;
	height: 0.9rem;
}

.ai-tool-call__details {
	padding: 0 0.6rem 0.6rem;
	display: flex;
	flex-direction: column;
	gap: 0.4rem;
}

.ai-tool-call__section {
	display: flex;
	flex-direction: column;
	gap: 0.15rem;
}

.ai-tool-call__section-title {
	font-size: 0.7rem;
	color: var(--color-text-tertiary);
}

.ai-tool-call__confirm {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
	padding-top: 0.25rem;
	border-top: 1px solid var(--color-divider);
}

.ai-tool-call__confirm-text {
	margin: 0;
	font-size: 0.8rem;
	color: var(--color-text-default);
}

.ai-tool-call__confirm-actions {
	display: flex;
	gap: 0.5rem;
}

.ai-tool-call__confirm-actions > * {
	flex: 1;
}

.ai-tool-call__details pre {
	margin: 0;
	max-height: 12rem;
	overflow: auto;
	font-size: 0.7rem;
	line-height: 1.35;
	white-space: pre-wrap;
	word-break: break-word;
	background: var(--color-raised-bg);
	border-radius: 6px;
	padding: 0.4rem 0.5rem;
}

.ai-panel__chat-error-row {
	display: flex;
	align-items: center;
	gap: 0.4rem;
	padding: 0.25rem 0.75rem;
	flex-shrink: 0;
}

.ai-panel__chat-error {
	margin: 0;
	font-size: 0.75rem;
	color: var(--color-red, #e06060);
	flex: 1;
}

.ai-panel__input-row {
	display: flex;
	align-items: flex-end;
	gap: 0.5rem;
	padding: 0.5rem;
	border-top: 1px solid var(--color-divider);
	flex-shrink: 0;
}

.ai-panel__textarea {
	flex-grow: 1;
	resize: none;
	max-height: 6rem;
	padding: 0.5rem;
	border-radius: 8px;
	border: 1px solid var(--color-divider);
	background: var(--color-button-bg);
	color: var(--color-text-default);
	font: inherit;
	font-size: 0.85rem;
}

.ai-panel__textarea:focus {
	outline: none;
	border-color: var(--color-brand);
}
</style>
