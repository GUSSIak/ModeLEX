// apps/app-frontend/src/helpers/modlex-ai.ts
import { invoke } from '@tauri-apps/api/core'

// Идентификатор плагина Tauri — обязан быть kebab-case (ACL не пускает
// подчёркивания вообще), см. apps/app/src/api/modlex_ai.rs::init().
const AI_API_BASE = 'plugin:modlex-ai'

export interface AiToolCall {
	id: string
	type: string
	/** Бэкенд (AiToolCall::Serialize в modlex_ai.rs) всегда отдаёт вложенный
	 * OpenAI wire-формат, а не плоские name/arguments — иначе история
	 * ломается при отправке обратно в Groq/z.ai/Cloudflare. */
	function: {
		name: string
		/** Raw JSON string, as sent by the model — parse per-tool as needed. */
		arguments: string
	}
}

export interface AiChatMessage {
	role: 'system' | 'user' | 'assistant' | 'tool'
	/** Null when the assistant message only carries tool_calls. */
	content: string | null
	tool_calls?: AiToolCall[] | null
	/** Set on role: "tool" messages — which call this is the result of. */
	tool_call_id?: string | null
}

export interface AiModelInfo {
	id: string
}

/**
 * Discriminated union mirroring theseus::modlex_ai::AgentStepOutcome.
 * `reply` — the agent produced a final text answer (after any number of
 * auto-executed read-only tool calls). `pending_confirmation` — the model
 * called a state-changing (tier Confirm) tool; nothing has executed yet.
 * Show a confirm/deny UI for `pending_call` and resolve it via
 * `modlex_ai_agent_resume` before sending anything else.
 */
export type AgentStepOutcome =
	| { outcome: 'reply'; messages: AiChatMessage[]; text: string }
	| { outcome: 'pending_confirmation'; messages: AiChatMessage[]; pending_call: AiToolCall }

/**
 * Sends the WHOLE conversation history (including any prior tool_calls/tool
 * result messages) and gets back the next step. There's no server-side
 * session — always pass back exactly the `messages` array from the previous
 * outcome, with the new user message appended.
 */
export async function modlex_ai_agent_step(messages: AiChatMessage[]): Promise<AgentStepOutcome> {
	return await invoke(`${AI_API_BASE}|modlex_ai_agent_step`, { messages })
}

/**
 * Resolves a `pending_confirmation` outcome — pass back the SAME `messages`
 * array and `pending_call` it came with, plus the user's decision. Nothing
 * from `pending_call` ever executes unless `approved` is true.
 */
export async function modlex_ai_agent_resume(
	messages: AiChatMessage[],
	pendingCall: AiToolCall,
	approved: boolean,
): Promise<AgentStepOutcome> {
	return await invoke(`${AI_API_BASE}|modlex_ai_agent_resume`, {
		messages,
		pendingCall,
		approved,
	})
}

export async function modlex_ai_list_models(): Promise<AiModelInfo[]> {
	return await invoke(`${AI_API_BASE}|modlex_ai_list_models`)
}

export async function modlex_ai_send_bug_report(
	title: string,
	description: string,
	fields: [string, string][],
): Promise<void> {
	return await invoke(`${AI_API_BASE}|modlex_ai_send_bug_report`, {
		title,
		description,
		fields,
	})
}

export type KeyPingStatus = 'ok' | 'rateLimited' | 'invalidKey' | 'error'

/** Field names are camelCase to match theseus::modlex_ai::KeyPingResult's
 * `#[serde(rename_all = "camelCase")]` wire format — NOT the Rust struct's
 * own (snake_case) field names. */
export interface KeyPingResult {
	provider: string
	keyPreview: string
	status: KeyPingStatus
	latencyMs: number
	detail: string | null
	quotaHint: string | null
}

/** Compile-time-only flag — see modlex_ai.rs's module doc. Only true in the
 * developer's own local build, never in an official/CI-built release. */
export async function modlex_ai_dev_ping_available(): Promise<boolean> {
	return await invoke(`${AI_API_BASE}|modlex_ai_dev_ping_available`)
}

/** Developer-only: pings every built-in shared key across all providers. */
export async function modlex_ai_ping_key_pool(): Promise<KeyPingResult[]> {
	return await invoke(`${AI_API_BASE}|modlex_ai_ping_key_pool`)
}

/** Pings only the caller's own z.ai key — safe for anyone, spends only their
 * own quota. */
export async function modlex_ai_ping_own_key(key: string): Promise<KeyPingResult> {
	return await invoke(`${AI_API_BASE}|modlex_ai_ping_own_key`, { key })
}
