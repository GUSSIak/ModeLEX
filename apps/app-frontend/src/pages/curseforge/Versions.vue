<template>
	<div>
		<ProjectPageVersions
			:loaders="loaders"
			:game-versions="gameVersions"
			:versions="versionRows"
			:project="project"
			:version-link="(version) => `/curseforge/${project.id}/version/${version.id}`"
		>
			<template #actions="{ version }">
				<IconButton
					v-tooltip="`Install`"
					type="quiet"
					label="Install"
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
				</IconButton>
				<ButtonLink
					v-tooltip="`Open in browser`"
					type="quiet"
					class="group-hover:!bg-button-bg"
					:href="`https://www.curseforge.com/minecraft/mc-mods/${project.slug}/files/${version.id}`"
					target="_blank"
				>
					<ExternalIcon />
				</ButtonLink>
			</template>
		</ProjectPageVersions>
	</div>
</template>

<script setup>
import { CheckIcon, DownloadIcon, ExternalIcon, RefreshCwIcon } from '@modrinth/assets'
import {
	ButtonLink,
	IconButton,
	injectNotificationManager,
	ProjectPageVersions,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

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

// ProjectPageVersions — общий Modrinth-компонент, ждёт форму реальной Modrinth
// версии (version_number/game_versions/loaders/version_type/date_published/
// downloads/files) — сырой CfFile этого не даёт вообще, отсюда были "?"/NaN/
// "No mod loader" на каждой строке. id держим тем же числом, что и в сыром
// CfFile — install(version.id) в Index.vue ищет файл через files.value.find(f
// => f.id === version), сравнение сломается, если тут превратить id в строку.
const RELEASE_TYPE_NAMES = { 1: 'release', 2: 'beta', 3: 'alpha' }
const LOADER_NAMES = ['forge', 'fabric', 'quilt', 'neoforge']

const versionRows = computed(() =>
	props.versions.map((f) => {
		const gameVersionTags = f.gameVersions || []
		const loaders = gameVersionTags
			.filter((v) => LOADER_NAMES.includes(v.toLowerCase()))
			.map((v) => v.toLowerCase())
		const gameVersions = gameVersionTags.filter((v) => !LOADER_NAMES.includes(v.toLowerCase()))
		return {
			id: f.id,
			name: f.displayName,
			version_number: f.displayName,
			game_versions: gameVersions,
			loaders,
			version_type: RELEASE_TYPE_NAMES[f.releaseType] || 'release',
			date_published: f.fileDate,
			downloads: 0,
			files: [{ filename: f.fileName, size: f.fileLength, primary: true }],
		}
	}),
)

const [loaders, gameVersions] = await Promise.all([
	get_loaders().catch(handleError).then(ref),
	get_game_versions().catch(handleError).then(ref),
])
</script>
