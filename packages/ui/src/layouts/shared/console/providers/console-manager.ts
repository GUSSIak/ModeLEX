import type { Mclogs } from '@modrinth/api-client'
import type { ComputedRef, Ref } from 'vue'

import { createContext } from '#ui/providers/create-context'

import type { LogLine, LogSource } from '../types'

export interface ConsoleManagerContext {
	logLines: Ref<LogLine[]>

	logSources?: ComputedRef<LogSource[]>
	activeLogSourceIndex?: Ref<number>

	sendCommand?: (cmd: string) => void
	showCommandInput?: boolean | Ref<boolean> | ComputedRef<boolean>
	disableCommandInput?: boolean | Ref<boolean> | ComputedRef<boolean>
	disableCommandInputTooltip?: string | Ref<string | undefined> | ComputedRef<string | undefined>

	loading?: Ref<boolean> | ComputedRef<boolean>

	onClear?: () => void
	clearDisabled?: Ref<boolean> | ComputedRef<boolean>
	clearDisabledTooltip?: string | Ref<string | undefined> | ComputedRef<string | undefined>
	onDelete?: () => Promise<void>
	deleteDisabled?: Ref<boolean> | ComputedRef<boolean>
	deleteDisabledTooltip?: string

	shareDisabled?: Ref<boolean> | ComputedRef<boolean>

	emptyStateType?: 'server' | 'instance'
	/**
	 * Кастомизация пустого экрана консоли (текст/размер/зазор матричного дождя).
	 * Именно refs/computed, а не голые значения — чтобы правки в Настройках
	 * применялись сразу на уже открытой вкладке, без её переоткрытия (см.
	 * layout.vue, где они разворачиваются через .value перед передачей в
	 * BaseTerminal).
	 */
	emptyStateText?: Ref<string | undefined> | ComputedRef<string | undefined>
	emptyStateScale?: Ref<number | undefined> | ComputedRef<number | undefined>
	emptyStateLetterGap?: Ref<number | undefined> | ComputedRef<number | undefined>
	emptyStateFillChar?: Ref<string | undefined> | ComputedRef<string | undefined>
	emptyStateRainChars?: Ref<string | undefined> | ComputedRef<string | undefined>
	emptyStateRainEnabled?: Ref<boolean | undefined> | ComputedRef<boolean | undefined>
	emptyStateFillColor?: Ref<string | undefined> | ComputedRef<string | undefined>
	emptyStateRainColor?: Ref<string | undefined> | ComputedRef<string | undefined>
	consoleBackgroundColor?: Ref<string | undefined> | ComputedRef<string | undefined>

	crashAnalysis?: Ref<Mclogs.Insights.v1.InsightsResponse | null>
	onDismissCrash?: () => void
}

export const [injectConsoleManager, provideConsoleManager] = createContext<ConsoleManagerContext>(
	'ConsolePageLayout',
	'consoleManagerContext',
)
