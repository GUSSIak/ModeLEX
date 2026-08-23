<template>
	<div
		class="flex h-full w-full flex-col bg-surface-2 overflow-hidden rounded-[20px] border border-solid border-surface-4"
		:style="
			backgroundColor ? { '--modlex-console-bg': backgroundColor, backgroundColor } : undefined
		"
	>
		<div ref="wrapperRef" class="relative min-h-0 flex-1 overflow-hidden pb-2 pt-1">
			<div ref="containerRef" class="size-full" />
			<Transition name="terminal-loading-fade">
				<div
					v-if="loading"
					class="pointer-events-none absolute inset-0 z-20 animate-bpulse bg-surface-3"
					aria-hidden="true"
				/>
			</Transition>
			<div v-if="!isAtBottom" class="absolute bottom-4 right-4 z-10">
				<IconButton size="xl" label="Scroll to bottom" class="!shadow-2xl" @click="scrollToBottom">
					<ChevronDownIcon />
				</IconButton>
			</div>
		</div>
		<div
			v-if="showInput"
			ref="inputRef"
			class="border-t border-solid border-b-0 border-x-0 border-surface-4 bg-surface-3 p-4"
		>
			<StyledInput
				v-model="commandInput"
				v-tooltip="disableInput ? disableInputTooltip : undefined"
				:icon="TerminalSquareIcon"
				:placeholder="disableInput ? disabledInputPlaceholder : 'Send a command'"
				:disabled="disableInput"
				wrapper-class="w-full"
				input-class="!h-10"
				@keydown.enter="submitCommand"
			/>
		</div>
	</div>
</template>

<script setup lang="ts">
import { ChevronDownIcon, TerminalSquareIcon } from '@modrinth/assets'
import type { Terminal } from '@xterm/xterm'
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'

import { IconButton } from '#ui/components/base/buttons'
import StyledInput from '#ui/components/base/StyledInput.vue'
import { useTerminal } from '#ui/composables/terminal'

const props = withDefaults(
	defineProps<{
		scrollback?: number
		showInput?: boolean
		disableInput?: boolean
		disableInputTooltip?: string
		disabledInputPlaceholder?: string
		fullscreen?: boolean
		emptyStateType?: 'server' | 'instance'
		loading?: boolean
		/** Текст пустого экрана (матричный дождь). По умолчанию — "NO SIGNAL". */
		emptyStateText?: string
		/** Явный размер букв (1-6). Если не задан — автоподбор под размер терминала. */
		emptyStateScale?: number
		/** Доп. зазор между буквами в колонках терминала. */
		emptyStateLetterGap?: number
		/** Символ, которым закрашены сами буквы (один символ). По умолчанию "#". */
		emptyStateFillChar?: string
		/** Алфавит символов падающего "дождя". По умолчанию — катакана + цифры. */
		emptyStateRainChars?: string
		/** Идёт ли анимация дождя. Если false — статичный ASCII-волк вместо неё. */
		emptyStateRainEnabled?: boolean
		/** Цвет букв-дыр в матричном дожде (hex). По умолчанию — тёмно-серый.
		 * Не влияет на арт волка — у него фиксированный цвет, см. WOLF_COLOR. */
		emptyStateFillColor?: string
		/** Цвет символов дождя (hex). По умолчанию — зелёный. */
		emptyStateRainColor?: string
		/** Фон терминала (hex). По умолчанию — из темы приложения. */
		backgroundColor?: string
	}>(),
	{
		scrollback: Infinity,
		showInput: false,
		disableInput: false,
		disableInputTooltip: undefined,
		disabledInputPlaceholder: 'Server is not running',
		fullscreen: false,
		emptyStateType: undefined,
		loading: false,
		emptyStateText: 'NO SIGNAL',
		emptyStateScale: undefined,
		emptyStateLetterGap: 2,
		emptyStateFillChar: '#',
		emptyStateRainChars: undefined,
		emptyStateRainEnabled: true,
		emptyStateFillColor: undefined,
		emptyStateRainColor: undefined,
		backgroundColor: undefined,
	},
)

// 5x7 dot-matrix font — используется как маска "дыр" в матричном дожде, чтобы
// буквы проступали как пустое место среди падающих символов. Неизвестные
// символы (вне A-Z/0-9/пробела/дефиса) тихо схлопываются в пробел — см.
// `DOT_FONT[ch] ?? DOT_FONT[' ']` в buildNoSignalMask/computeLetterScale.
const DOT_FONT: Record<string, string[]> = {
	A: ['01110', '10001', '10001', '11111', '10001', '10001', '10001'],
	B: ['11110', '10001', '10001', '11110', '10001', '10001', '11110'],
	C: ['01111', '10000', '10000', '10000', '10000', '10000', '01111'],
	D: ['11110', '10001', '10001', '10001', '10001', '10001', '11110'],
	E: ['11111', '10000', '10000', '11110', '10000', '10000', '11111'],
	F: ['11111', '10000', '10000', '11110', '10000', '10000', '10000'],
	G: ['01110', '10001', '10000', '10011', '10001', '10001', '01111'],
	H: ['10001', '10001', '10001', '11111', '10001', '10001', '10001'],
	I: ['11111', '00100', '00100', '00100', '00100', '00100', '11111'],
	J: ['00111', '00010', '00010', '00010', '00010', '10010', '01100'],
	K: ['10001', '10010', '10100', '11000', '10100', '10010', '10001'],
	L: ['10000', '10000', '10000', '10000', '10000', '10000', '11111'],
	M: ['10001', '11011', '10101', '10101', '10001', '10001', '10001'],
	N: ['10001', '11001', '10101', '10011', '10001', '10001', '10001'],
	O: ['01110', '10001', '10001', '10001', '10001', '10001', '01110'],
	P: ['11110', '10001', '10001', '11110', '10000', '10000', '10000'],
	Q: ['01110', '10001', '10001', '10001', '10101', '10010', '01101'],
	R: ['11110', '10001', '10001', '11110', '10100', '10010', '10001'],
	S: ['01111', '10000', '10000', '01110', '00001', '00001', '11110'],
	T: ['11111', '00100', '00100', '00100', '00100', '00100', '00100'],
	U: ['10001', '10001', '10001', '10001', '10001', '10001', '01110'],
	V: ['10001', '10001', '10001', '10001', '10001', '01010', '00100'],
	W: ['10001', '10001', '10001', '10101', '10101', '10101', '01010'],
	X: ['10001', '10001', '01010', '00100', '01010', '10001', '10001'],
	Y: ['10001', '10001', '01010', '00100', '00100', '00100', '00100'],
	Z: ['11111', '00001', '00010', '00100', '01000', '10000', '11111'],
	'0': ['01110', '10001', '10011', '10101', '11001', '10001', '01110'],
	'1': ['00100', '01100', '00100', '00100', '00100', '00100', '01110'],
	'2': ['01110', '10001', '00001', '00010', '00100', '01000', '11111'],
	'3': ['11111', '00010', '00100', '00010', '00001', '10001', '01110'],
	'4': ['00010', '00110', '01010', '10010', '11111', '00010', '00010'],
	'5': ['11111', '10000', '11110', '00001', '00001', '10001', '01110'],
	'6': ['00110', '01000', '10000', '11110', '10001', '10001', '01110'],
	'7': ['11111', '00001', '00010', '00100', '01000', '01000', '01000'],
	'8': ['01110', '10001', '10001', '01110', '10001', '10001', '01110'],
	'9': ['01110', '10001', '10001', '01111', '00001', '00010', '01100'],
	'-': ['00000', '00000', '00000', '11111', '00000', '00000', '00000'],
	' ': ['000', '000', '000', '000', '000', '000', '000'],
}
const FONT_HEIGHT = 7
// Отступ внутри "логики" шрифта — держим маленьким, чтобы не съедать масштаб букв
const LETTER_GAP_PX = 1

function getEmptyStateText(): string {
	return (props.emptyStateText || 'NO SIGNAL').toUpperCase()
}

function getLetterGap(): number {
	return props.emptyStateLetterGap ?? 2
}
// Цвет, которым рисуются сами буквы (плотный тёмный силуэт, а не дыра) —
// сам символ теперь настраиваемый, см. getFillChar(). Дефолты — стандартная
// 16-цветная ANSI-палитра; при заданном hex-цвете переключаемся на 24-битный
// truecolor (\x1B[38;2;R;G;Bm), который xterm.js поддерживает нативно.
const LETTER_FILL_COLOR = '\x1B[90m' // тёмно-серый (дефолт) — на фоне дождя это "дыра"
const RAIN_COLOR = '\x1B[32m' // зелёный (дефолт)
const RAIN_COLOR_BOLD = '\x1B[1;32m'

function hexToAnsiTruecolor(hex: string, bold = false): string | null {
	const match = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex.trim())
	if (!match) return null
	const r = parseInt(match[1], 16)
	const g = parseInt(match[2], 16)
	const b = parseInt(match[3], 16)
	return `${bold ? '\x1B[1m' : ''}\x1B[38;2;${r};${g};${b}m`
}

// Волк не участвует в матричном дожде и не окружён другими символами, так что
// его нельзя красить тем же (нарочно тёмным, "дырчатым") цветом, что и буквы
// в дожде — на чёрном фоне он попросту пропадает. Фиксированный светлый
// truecolor, ни от темы (CSS-переменных), ни от пользовательского цвета
// заполнения не зависит — иначе кастомная тёмная тема/акцент опять сделает
// волка невидимым.
const WOLF_COLOR = hexToAnsiTruecolor('#c7cbd4')!

function getFillColorEscape(): string {
	return (
		(props.emptyStateFillColor && hexToAnsiTruecolor(props.emptyStateFillColor)) ||
		LETTER_FILL_COLOR
	)
}

function getRainColorEscape(bold: boolean): string {
	if (props.emptyStateRainColor) {
		const truecolor = hexToAnsiTruecolor(props.emptyStateRainColor, bold)
		if (truecolor) return truecolor
	}
	return bold ? RAIN_COLOR_BOLD : RAIN_COLOR
}

// ASCII-волк (брайлевская мозаика) — статичная замена дождю, когда
// emptyStateRainEnabled выключен. Пробелы внутри рисунка — символ "пустой
// брайль" (U+2800), а не обычный пробел: в моноширинных шрифтах терминала
// оба занимают одну и ту же ячейку, но брайль гарантированно не схлопывается
// иначе при копировании/рендере в некоторых окружениях — сохраняем как есть.
const SLEEPING_WOLF_ART = [
	'⠀⠀⠀⢀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀',
	'⠀⠀⣸⢏⢙⣦⡀⠀⠀⠀⠀⠀⠀⠀⠀⣀⢴⠟⠙⠀⠀⠀⠀⠀⠀⠀⠀⠀',
	'⠀⠀⡏⠉⠑⢄⠈⠳⣄⠀⢀⣀⣀⢀⠞⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀',
	'⠀⣶⣧⣤⠄⠀⠉⠢⡈⠋⠁⠀⠀⠉⠓⠦⣄⡀⣦⣀⡠⠤⣄⠀⠀⠀⠀⠀',
	'⠈⣟⢯⣇⠁⠀⠀⠀⠈⢶⣆⠀⠀⠀⠀⠀⣴⠽⠚⣉⣠⠇⣾⠀⠀⠀⠀⠀',
	'⠀⠙⣆⠹⠀⠀⠀⠀⠀⠈⠙⠓⠀⠀⠀⠀⠠⠔⠋⣡⠋⢘⡏⠀⠀⠀⠀⠀',
	'⠀⠀⠈⢦⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠠⠴⠋⠀⢀⢾⡀⠀⠀⠀⠀⠀',
	'⠀⠀⠀⠈⢧⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⡠⠋⢀⣼⣋⠀⠀⠀⠀',
	'⠀⠀⠀⠀⠈⢧⠀⠀⠀⠀⠀⠀⢀⢰⣇⠀⠀⠀⣺⠆⠀⠀⠀⠀⠙⠢⣄⠀',
	'⠀⠀⠀⠀⠀⢸⢠⠀⢀⠀⠰⡄⠀⡷⡇⠀⠀⠉⠁⠀⠀⠀⠀⢐⠚⠻⣍⠀',
	'⠀⠀⠀⠀⠀⠾⠛⡦⡿⡷⢷⡟⠢⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠉⠉⠛⠂',
	'⠀⠀⠀⠀⠀⠀⠀⢷⠈⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀',
	'⠀⠀⠀⠀⠀⠀⠀⢸⣶⡏⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀',
	'⠀⠀⠀⠀⠀⠀⠀⠈⢩⠇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀',
	'⠀⠀⠀⠀⠀⠀⠀⠀⠈⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀',
	'⠀⠀⠀⠀⠀⠀⠀⠀⠀⢱⣿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀',
	'⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠙⡄⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀',
]

// Подпись под артом — только на пустом экране, пока нет реального лога.
const SLEEPING_WOLF_CAPTION = [
	"It's empty here for now...",
	'Logs will appear once you launch the game',
]

const RAIN_CHARS_DEFAULT = 'ｦｱｳｴｵｶｷｹｺｻｼｽｾｿﾀﾂﾃﾅﾆﾇﾈﾊﾋﾎﾏﾐﾑﾒﾓﾔﾕﾗﾘﾜ0123456789'

const TICK_MS = 80
const INTRO_MS = 1800 // плавное появление дождя при старте, как в оригинале
const BOTTOM_FADE_ROWS = 10 // зона плавного угасания дождя у самого низа

function getFillChar(): string {
	return (props.emptyStateFillChar || '#').slice(0, 1) || '#'
}

function getRainChars(): string {
	return props.emptyStateRainChars && props.emptyStateRainChars.length > 0
		? props.emptyStateRainChars
		: RAIN_CHARS_DEFAULT
}

function randomRainChar(): string {
	const chars = getRainChars()
	return chars[Math.floor(Math.random() * chars.length)]
}

/** Подбирает масштаб букв так, чтобы слово занимало большую часть экрана, но помещалось. */
function computeLetterScale(cols: number, rows: number): { scaleX: number; scaleY: number } {
	const letterGap = getLetterGap()
	const letters = getEmptyStateText().split('')
	const gapCount = letters.length - 1

	// Явный размер из настроек — используем как есть, без автоподбора под контейнер
	// (нужно для превью в настройках, где важно видеть реальный "размер точки", а не
	// то, что подгонится под маленькую рамку предпросмотра).
	if (props.emptyStateScale) {
		const scaleY = Math.max(1, Math.min(6, Math.round(props.emptyStateScale)))
		return { scaleX: Math.max(1, scaleY * 2), scaleY }
	}

	const baseWidth =
		letters.reduce((sum, ch) => sum + (DOT_FONT[ch]?.[0]?.length ?? 0) + LETTER_GAP_PX, 0) -
		LETTER_GAP_PX
	// Фиксированные зазоры между буквами вычитаем из бюджета ширины заранее,
	// чтобы они не "съедали" сам масштаб букв — только доступное место под них
	const availableCols = Math.max(1, cols - gapCount * letterGap)

	const maxScaleYByHeight = Math.max(1, Math.floor((rows * 0.55) / FONT_HEIGHT))
	const maxScaleXByWidth = Math.max(1, Math.floor((availableCols * 0.85) / baseWidth))

	// Символы в моноширинном терминале примерно вдвое выше, чем широкие,
	// поэтому по колонкам масштабируем сильнее — чтобы "пиксели" были почти квадратными.
	const scaleY = Math.min(maxScaleYByHeight, 6)
	const scaleX = Math.min(maxScaleXByWidth, scaleY * 2)

	return { scaleX: Math.max(1, scaleX), scaleY: Math.max(1, scaleY) }
}

/** Строит по центру увеличенную маску букв под текущий размер терминала. */
function buildNoSignalMask(
	cols: number,
	rows: number,
): { mask: boolean[][]; top: number; bottom: number } {
	const mask: boolean[][] = Array.from({ length: rows }, () => Array(cols).fill(false))
	const { scaleX, scaleY } = computeLetterScale(cols, rows)
	const letterGap = getLetterGap()

	const letters = getEmptyStateText().split('')
	const gapCount = letters.length - 1
	const wordWidth =
		letters.reduce((sum, ch) => sum + (DOT_FONT[ch]?.[0]?.length ?? 0) * scaleX, 0) +
		gapCount * (LETTER_GAP_PX * scaleX + letterGap)
	const wordHeight = FONT_HEIGHT * scaleY

	if (cols < wordWidth + 2 || rows < wordHeight + 2) {
		return { mask, top: -1, bottom: -1 }
	}

	const startCol = Math.floor((cols - wordWidth) / 2)
	const startRow = Math.floor((rows - wordHeight) / 2)

	let col = startCol
	for (const ch of letters) {
		const glyph = DOT_FONT[ch] ?? DOT_FONT[' ']
		const glyphWidth = glyph[0].length
		for (let r = 0; r < FONT_HEIGHT; r++) {
			for (let c = 0; c < glyphWidth; c++) {
				if (glyph[r][c] !== '1') continue
				for (let dy = 0; dy < scaleY; dy++) {
					for (let dx = 0; dx < scaleX; dx++) {
						const targetRow = startRow + r * scaleY + dy
						const targetCol = col + c * scaleX + dx
						if (targetRow >= 0 && targetRow < rows && targetCol >= 0 && targetCol < cols) {
							mask[targetRow][targetCol] = true
						}
					}
				}
			}
		}
		col += glyphWidth * scaleX + LETTER_GAP_PX * scaleX + letterGap
	}

	return { mask, top: startRow, bottom: startRow + wordHeight - 1 }
}

interface RainColumn {
	head: number
	speed: number
	length: number
}

// Насколько строк выше/ниже слова "утолщается" фон, чтобы буквы точно читались
const AMBIENT_MARGIN_ROWS = 2
// Базовый шанс наличия символа в зоне букв (не 100%, чтобы фон чуть "дышал")
const AMBIENT_DENSITY = 0.99

let rainTimer: ReturnType<typeof setInterval> | null = null
let rainMask: boolean[][] = []
let rainCols = 0
let rainRows = 0
let rainStartRow = 1
let rainColumns: RainColumn[] = []
let rainCellChars: string[][] = []
let rainStartedAt = 0
let ambientTop = -1
let ambientBottom = -1

// Обычный lineHeight терминала (запоминается при инициализации) против более
// плотного — только для статичного арта волка, где стандартный интервал между
// строк растягивает рисунок и рвёт форму. Для дождя/реальных логов не трогаем.
let normalLineHeight = 1.5
const WOLF_LINE_HEIGHT = 1.05

function setTerminalLineHeight(value: number) {
	if (!terminal.value) return
	if (terminal.value.options.lineHeight === value) return
	terminal.value.options.lineHeight = value
	rawFit()
}

function spawnColumn(rows: number): RainColumn {
	return {
		head: -Math.floor(Math.random() * rows * 0.3),
		speed: 0.35 + Math.random() * 0.5,
		length: 8 + Math.floor(Math.random() * 16),
	}
}

function renderRainFrame() {
	if (!terminal.value) return

	const introFactor = Math.min(1, (Date.now() - rainStartedAt) / INTRO_MS)

	let frame = '\x1B[?25l' // скрыть курсор на время анимации
	for (let r = 0; r < rainRows; r++) {
		frame += `\x1B[${rainStartRow + r};1H\x1B[2K`
		let line = ''
		// плавное угасание у самого низа доступной области — эффект стекающей краски
		const bottomFade = Math.min(1, Math.max(0, (rainRows - 1 - r) / BOTTOM_FADE_ROWS))

		const inAmbientZone = ambientTop >= 0 && r >= ambientTop && r <= ambientBottom

		for (let c = 0; c < rainCols; c++) {
			if (rainMask[r]?.[c]) {
				line += `${getFillColorEscape()}${getFillChar()}\x1B[0m`
				continue
			}

			const column = rainColumns[c]
			const headRow = Math.floor(column.head)
			const inTrail = r <= headRow && r >= headRow - column.length
			const distFromHead = headRow - r
			const trailFade = inTrail ? 1 - distFromHead / column.length : 0
			const rainVisibility = trailFade * introFactor * bottomFade

			// В зоне букв всегда держим плотный (но чуть "дышащий") фон — иначе
			// дыры-буквы теряются среди естественных пропусков разреженного дождя.
			const visibility = inAmbientZone
				? Math.max(rainVisibility, AMBIENT_DENSITY * introFactor)
				: rainVisibility

			if (Math.random() > visibility) {
				line += ' '
				continue
			}

			// символ клетки почти не меняется каждый кадр — только изредка,
			// чтобы дождь мягко "жил", а не мерцал резко на весь экран
			if (!rainCellChars[r][c] || Math.random() < 0.06) {
				rainCellChars[r][c] = randomRainChar()
			}
			const ch = rainCellChars[r][c]

			if (inTrail && distFromHead === 0) {
				line += `\x1B[1;97m${ch}\x1B[0m`
			} else if (inTrail && trailFade > 0.6) {
				line += `${getRainColorEscape(true)}${ch}\x1B[0m`
			} else {
				line += `${getRainColorEscape(false)}${ch}\x1B[0m`
			}
		}
		frame += line
	}
	terminal.value.write(frame)

	for (const column of rainColumns) {
		column.head += column.speed
		if (column.head - column.length > rainRows) {
			Object.assign(column, spawnColumn(rainRows))
		}
	}
}

function startMatrixRain(startRow: number) {
	stopMatrixRain()
	if (!terminal.value) return
	setTerminalLineHeight(normalLineHeight)

	rainStartRow = startRow
	rainCols = terminal.value.cols
	rainRows = Math.max(0, terminal.value.rows - startRow + 1)
	if (rainRows <= 0) return

	const built = buildNoSignalMask(rainCols, rainRows)
	rainMask = built.mask
	ambientTop = built.top >= 0 ? Math.max(0, built.top - AMBIENT_MARGIN_ROWS) : -1
	ambientBottom =
		built.bottom >= 0 ? Math.min(rainRows - 1, built.bottom + AMBIENT_MARGIN_ROWS) : -1
	rainColumns = Array.from({ length: rainCols }, () => spawnColumn(rainRows))
	rainCellChars = Array.from({ length: rainRows }, () => Array(rainCols).fill(''))
	rainStartedAt = Date.now()

	rainTimer = setInterval(renderRainFrame, TICK_MS)
}

function stopMatrixRain() {
	if (rainTimer) {
		clearInterval(rainTimer)
		rainTimer = null
	}
	if (terminal.value) {
		terminal.value.write('\x1B[?25h') // вернуть курсор
	}
}

/** Статичная замена дождю — рисуется один раз, без анимации/таймера. */
function writeSleepingWolf(startRow: number) {
	if (!terminal.value) return
	setTerminalLineHeight(WOLF_LINE_HEIGHT)

	const cols = terminal.value.cols
	const rows = Math.max(0, terminal.value.rows - startRow + 1)
	if (rows <= 0) return

	const artWidth = Math.max(...SLEEPING_WOLF_ART.map((line) => line.length))
	const artHeight = SLEEPING_WOLF_ART.length
	const captionGap = 1
	const totalHeight = artHeight + captionGap + SLEEPING_WOLF_CAPTION.length
	const startCol = Math.max(0, Math.floor((cols - artWidth) / 2))
	const artStartRow = Math.max(0, Math.floor((rows - totalHeight) / 2))
	const captionStartRow = artStartRow + artHeight + captionGap

	const color = WOLF_COLOR
	let frame = '\x1B[?25l'
	for (let r = 0; r < rows; r++) {
		frame += `\x1B[${startRow + r};1H\x1B[2K`
		const artLine = SLEEPING_WOLF_ART[r - artStartRow]
		if (artLine) {
			frame += ' '.repeat(startCol) + color + artLine + '\x1B[0m'
			continue
		}
		const captionLine = SLEEPING_WOLF_CAPTION[r - captionStartRow]
		if (captionLine) {
			const captionCol = Math.max(0, Math.floor((cols - captionLine.length) / 2))
			frame += ' '.repeat(captionCol) + color + captionLine + '\x1B[0m'
		}
	}
	terminal.value.write(frame)
}

const emit = defineEmits<{
	command: [command: string]
	ready: [terminal: Terminal]
}>()

const containerRef = ref<HTMLElement | null>(null)
const wrapperRef = ref<HTMLElement | null>(null)
const inputRef = ref<HTMLElement | null>(null)
const commandInput = ref('')

const snappedHeight = ref<number | null>(null)

const showingEmptyState = ref(false)

const {
	terminal,
	searchAddon,
	isAtBottom,
	write,
	writeln,
	clear,
	reset,
	fit: rawFit,
	scrollToBottom,
} = useTerminal({
	container: containerRef,
	scrollback: props.scrollback,
	backgroundColor: () => props.backgroundColor,
	onReady: (term) => {
		normalLineHeight = term.options.lineHeight ?? normalLineHeight
		nextTick(() => {
			snapToRows()
		})
		term.onResize(() => {
			// Полная переотрисовка вместо простого рестарта дождя — иначе после
			// возврата из полноэкранного режима верхняя часть терминала у xterm
			// может остаться "замороженной" со старого рендера.
			if (showingEmptyState.value) nextTick(() => writeEmptyState())
		})
		emit('ready', term)
	},
})

function writeEmptyState() {
	if (!terminal.value || !props.emptyStateType) return
	terminal.value.reset()
	showingEmptyState.value = true
	nextTick(() => {
		if (props.emptyStateRainEnabled) {
			startMatrixRain(1)
		} else {
			stopMatrixRain()
			writeSleepingWolf(1)
		}
	})
}

function clearEmptyState() {
	if (!showingEmptyState.value) return
	stopMatrixRain()
	// Флаг сбрасываем до setTerminalLineHeight — оно может синхронно вызвать
	// resize у xterm, а обработчик onResize перерисовывает пустой экран, только
	// пока showingEmptyState ещё true.
	showingEmptyState.value = false
	setTerminalLineHeight(normalLineHeight)
	terminal.value?.reset()
}

// Живой предпросмотр в настройках меняет текст/размер/зазор на лету — если
// пустой экран сейчас показан, перерисовываем его с новыми параметрами.
// Дебаунс — без него перетаскивание слайдера полностью перезапускает дождь
// (новое интро-появление, новые случайные колонки) на каждый тик, что
// выглядит как хаотичное мерцание, а не плавная подстройка размера.
let emptyStateRedrawTimer: ReturnType<typeof setTimeout> | null = null
watch(
	() => [
		props.emptyStateText,
		props.emptyStateScale,
		props.emptyStateLetterGap,
		props.emptyStateFillChar,
		props.emptyStateRainChars,
		props.emptyStateRainEnabled,
		props.emptyStateFillColor,
		props.emptyStateRainColor,
	],
	() => {
		if (!showingEmptyState.value) return
		if (emptyStateRedrawTimer) clearTimeout(emptyStateRedrawTimer)
		emptyStateRedrawTimer = setTimeout(() => {
			emptyStateRedrawTimer = null
			writeEmptyState()
		}, 200)
	},
)

function getWrapperMargins() {
	if (!wrapperRef.value) return 0
	const style = getComputedStyle(wrapperRef.value)
	return parseFloat(style.marginTop) + parseFloat(style.marginBottom)
}

function snapToRows() {
	if (!props.fullscreen) {
		snappedHeight.value = null
		return
	}
	const screen = containerRef.value?.querySelector('.xterm-screen') as HTMLElement | null
	if (!screen) {
		snappedHeight.value = null
		return
	}
	const inputH = inputRef.value?.offsetHeight ?? 0
	const borderW = 2
	snappedHeight.value = screen.offsetHeight + getWrapperMargins() + inputH + borderW
}

let resizeDebounce: ReturnType<typeof setTimeout> | null = null

function handleWindowResize() {
	if (!props.fullscreen) return
	if (resizeDebounce) clearTimeout(resizeDebounce)
	snappedHeight.value = null
	resizeDebounce = setTimeout(() => {
		rawFit()
		nextTick(() => snapToRows())
	}, 50)
}

function handleDocumentPointerDown(event: PointerEvent) {
	if (!terminal.value?.hasSelection()) return
	const target = event.target as Node | null
	if (target && containerRef.value?.contains(target)) return
	terminal.value.clearSelection()
}

function handleDocumentKeyDown(event: KeyboardEvent) {
	if (!event.metaKey || event.key.toLowerCase() !== 'a') return
	const target = event.target as Node | null
	const active = document.activeElement
	if (
		!(target && containerRef.value?.contains(target)) &&
		!(active && containerRef.value?.contains(active))
	) {
		return
	}

	event.preventDefault()
	terminal.value?.selectAll()
}

onMounted(() => {
	window.addEventListener('resize', handleWindowResize)
	document.addEventListener('pointerdown', handleDocumentPointerDown)
	document.addEventListener('keydown', handleDocumentKeyDown, true)
})

onBeforeUnmount(() => {
	window.removeEventListener('resize', handleWindowResize)
	document.removeEventListener('pointerdown', handleDocumentPointerDown)
	document.removeEventListener('keydown', handleDocumentKeyDown, true)
	if (resizeDebounce) clearTimeout(resizeDebounce)
	if (emptyStateRedrawTimer) clearTimeout(emptyStateRedrawTimer)
	stopMatrixRain()
})

function fit() {
	rawFit()
	snapToRows()
}

watch(
	() => props.fullscreen,
	() => {
		if (props.fullscreen) {
			nextTick(() => {
				rawFit()
				nextTick(() => snapToRows())
			})
		} else {
			snappedHeight.value = null
		}
	},
)

const submitCommand = () => {
	if (props.disableInput) return
	const cmd = commandInput.value.trim()
	if (!cmd) return
	emit('command', cmd)
	commandInput.value = ''
}

defineExpose({
	write,
	writeln,
	clear,
	reset,
	fit,
	scrollToBottom,
	terminal,
	searchAddon,
	isAtBottom,
	commandInput,
	showingEmptyState,
	writeEmptyState,
	clearEmptyState,
})
</script>

<style>
@keyframes bpulse {
	50% {
		filter: brightness(75%);
	}
}
.animate-bpulse {
	animation: bpulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite;
}

.xterm {
	height: 100% !important;
}

.xterm-viewport {
	background-color: var(--modlex-console-bg, var(--surface-2)) !important;
}

.xterm .xterm-screen {
	width: 100%;
	margin-left: 8px;
	margin-right: auto;
}

.xterm .xterm-rows {
	position: relative;
	z-index: 7;
}

.xterm .xterm-decoration-container {
	overflow: visible !important;
}

.xterm .xterm-decoration-container > div {
	box-sizing: content-box !important;
	margin-left: -12px !important;
	padding-left: 12px !important;
	padding-right: 12px !important;
}

.xterm-scrollable-element > .scrollbar.vertical {
	width: 8px !important;
}

.xterm-scrollable-element > .scrollbar.vertical > div {
	width: 6px !important;
	border-radius: 8px !important;
	contain: layout style !important;
}

.terminal-loading-fade-enter-active,
.terminal-loading-fade-leave-active {
	transition: opacity 250ms ease-in-out;
}

.terminal-loading-fade-enter-from,
.terminal-loading-fade-leave-to {
	opacity: 0;
}
</style>
