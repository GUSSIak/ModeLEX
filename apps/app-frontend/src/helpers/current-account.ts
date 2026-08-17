import { ref } from 'vue'

import { get_default_user } from '@/helpers/auth'

export const currentAccountId = ref<string | undefined>()

export async function refreshCurrentAccountId() {
	currentAccountId.value = await get_default_user()
}
