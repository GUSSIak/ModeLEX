import { ref } from 'vue'

// Component-local refs go stale across pages in this codebase (see memory
// workflow_shared_account_state) — ModlexAiAgent.vue is a singleton mounted
// once in App.vue, but error toasts/crash screens live on arbitrary other
// pages, so the trigger has to be a shared module-level ref, not a prop/event
// that assumes a common ancestor.
export interface AiAgentHelpRequest {
	message: string
	requestId: number
}

export const pendingAiAgentHelpRequest = ref<AiAgentHelpRequest | null>(null)

let nextRequestId = 0

/** Opens the ModLEX AI agent panel and sends `message` as if the user typed
 * it. Call from anywhere (error toasts, crash screens) — ModlexAiAgent.vue
 * watches this ref. */
export function requestAiAgentHelp(message: string) {
	pendingAiAgentHelpRequest.value = { message, requestId: ++nextRequestId }
}
