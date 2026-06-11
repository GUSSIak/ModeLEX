<template>
	<div class="settings-section">
		<h2 class="settings-section__title">
			{{ title }}
		</h2>
		<div class="settings-section__content">
			<slot />
		</div>
	</div>
</template>

<script setup lang="ts">
	defineProps<{
	title: string
	}>()
</script>

<style scoped lang="scss">
	.settings-section {
		&:not(:last-child) {
			margin-bottom: 2rem;
			padding-bottom: 1.5rem;
			border-bottom: 1px solid var(--color-divider);
		}

		&__title {
			font-size: 1.25rem;
			font-weight: 600;
			margin-bottom: 1rem;
			color: var(--color-contrast);
		}

		&__content {
			display: flex;
			flex-direction: column;
			gap: 1rem;
		}
	}
</style>
