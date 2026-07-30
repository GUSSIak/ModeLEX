<template>
	<Card>
		<ProjectPageDescription :description="project.body || 'No description available'" />
	</Card>
</template>

<script setup>
import { Card, ProjectPageDescription } from '@modrinth/ui'

defineProps({
	project: {
		type: Object,
		default: () => ({}),
	},
})
</script>
