<template>
	<Admonition type="info" :header="formatMessage(messages.offlineMultiplayerQuirkHeader)">
		{{ formatMessage(messages.offlineMultiplayerQuirkBody) }}
	</Admonition>
</template>

<script setup lang="ts">
import { Admonition, useVIntl } from '@modrinth/ui'

import { instanceAdmonitionsMessages as messages } from './messages'

const { formatMessage } = useVIntl()
</script>
