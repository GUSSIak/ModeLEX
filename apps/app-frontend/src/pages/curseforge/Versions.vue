<template>
	<div>
		<ProjectPageVersions
			:loaders="loaders"
			:game-versions="gameVersions"
			:versions="versions"
			:project="project"
			:version-link="(version) => `/curseforge/${project.id}/version/${version.id}`"
		>
			<template #actions="{ version }">
				<ButtonStyled circular type="transparent">
					<button
						v-tooltip="`Install`"
						:class="{
							'group-hover:!bg-brand group-hover:[&>svg]:!text-brand-inverted':
								!isInstalledVersion(version),
						}"
						:disabled="installing || isInstalledVersion(version)"
						@click.stop="() => install(version.id)"
					>
						<DownloadIcon v-if="!installed" />
						<RefreshCwIcon v-else-if="!isInstalledVersion(version)" />
						<CheckIcon v-else />
					</button>
				</ButtonStyled>
				<ButtonStyled circular type="transparent">
					<a
						v-tooltip="`Open in browser`"
						class="group-hover:!bg-button-bg"
						:href="`https://www.curseforge.com/minecraft/mc-mods/${project.slug}/files/${version.id}`"
						target="_blank"
					>
						<ExternalIcon />
					</a>
				</ButtonStyled>
			</template>
		</ProjectPageVersions>
	</div>
</template>

<script setup>
import { CheckIcon, DownloadIcon, ExternalIcon, RefreshCwIcon } from '@modrinth/assets'
import { ButtonStyled, injectNotificationManager, ProjectPageVersions } from '@modrinth/ui'
import { ref } from 'vue'

import { get_game_versions, get_loaders } from '@/helpers/tags.js'

const props = defineProps({
	project: {
		type: Object,
		default: () => {},
	},
	versions: {
		type: Array,
		required: true,
	},
	install: {
		type: Function,
		required: true,
	},
	installed: {
		type: Boolean,
		default: null,
	},
	installing: {
		type: Boolean,
		default: false,
	},
	instance: {
		type: Object,
		default: null,
	},
	installedVersion: {
		type: String,
		default: null,
	},
})

const { handleError } = injectNotificationManager()

// CfFile.id — число, installedVersion приходит строкой, поэтому сравниваем через String()
function isInstalledVersion(version) {
	return props.installed && String(version.id) === props.installedVersion
}

const [loaders, gameVersions] = await Promise.all([
	get_loaders().catch(handleError).then(ref),
	get_game_versions().catch(handleError).then(ref),
])
</script>
