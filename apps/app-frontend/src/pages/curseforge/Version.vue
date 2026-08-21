<template>
	<div>
		<Card>
			<Breadcrumbs
				:current-title="version.displayName"
				:link-stack="[
					{
						href: `/curseforge/${route.params.id}/versions`,
						label: 'Versions',
					},
				]"
			/>
			<div class="version-title">
				<h2>{{ version.displayName }}</h2>
			</div>
			<div class="button-group">
				<Button
					type="colored"
					color="brand"
					native-type="button"
					:disabled="installing || isInstalledVersion"
					@click="() => install(version.id)"
				>
					<DownloadIcon v-if="!installed" />
					<RefreshCwIcon v-else-if="!isInstalledVersion" />
					<CheckIcon v-else />
					{{ installing ? 'Installing...' : isInstalledVersion ? 'Installed' : 'Install' }}
				</Button>
				<Button type="outlined" native-type="button">
					<ReportIcon />
					Report
				</Button>
				<ButtonLink
					type="outlined"
					:href="`https://www.curseforge.com/minecraft/mc-mods/${project.slug}/files/${version.id}`"
				>
					CurseForge website
					<ExternalIcon />
				</ButtonLink>
			</div>
		</Card>
		<div class="version-container">
			<div class="description-cards">
				<Card>
					<h3 class="card-title">Changelog</h3>
					<div class="markdown-body" v-html="version.changelog || 'No changelog provided'" />
				</Card>
				<Card>
					<h3 class="card-title">Files</h3>
					<Card class="file">
						<span class="label">
							<FileIcon />
							<span>
								<span class="title">
									{{ version.fileName }}
								</span>
								({{ formatBytes(version.fileLength) }})
							</span>
						</span>
						<Button
							type="colored"
							color="brand"
							class="download"
							native-type="button"
							:disabled="isInstalledVersion"
							@click="() => install(version.id)"
						>
							<DownloadIcon v-if="!isInstalledVersion" />
							<CheckIcon v-else />
							{{ isInstalledVersion ? 'Installed' : 'Install' }}
						</Button>
					</Card>
				</Card>
				<Card v-if="displayDependencies.length > 0">
					<h2>Dependencies</h2>
					<div v-for="dependency in displayDependencies" :key="dependency.title" class="dependency">
						<div v-if="dependency.link" class="btn" @click="router.push(dependency.link)">
							<Avatar size="sm" :src="dependency.icon" />
							<div>
								<span class="title"> {{ dependency.title }} </span> <br />
								<span> {{ dependency.subtitle }} </span>
							</div>
						</div>
						<div v-else class="dependency disabled" disabled="">
							<Avatar size="sm" :src="dependency.icon" />
							<div class="text">
								<div class="title">{{ dependency.title }}</div>
								<div>{{ dependency.subtitle }}</div>
							</div>
						</div>
					</div>
				</Card>
			</div>
			<Card class="metadata-card">
				<h3 class="card-title">Metadata</h3>
				<div class="metadata">
					<div class="metadata-item">
						<span class="metadata-label">Version Number</span>
						<span class="metadata-value">{{ version.displayName }}</span>
					</div>
					<div class="metadata-item">
						<span class="metadata-label">Game Versions</span>
						<span class="metadata-value">{{ version.gameVersions.join(', ') }}</span>
					</div>
					<div class="metadata-item">
						<span class="metadata-label">File Size</span>
						<span class="metadata-value">{{ formatBytes(version.fileLength) }}</span>
					</div>
					<div class="metadata-item">
						<span class="metadata-label">Release Date</span>
						<span class="metadata-value">
							{{ formatDateTime(new Date(version.fileDate)) }}
						</span>
					</div>
					<div v-if="author" class="metadata-item">
						<span class="metadata-label">Author</span>
						<div class="metadata-value author">
							<Avatar size="sm" :src="author.avatar_url" circle />
							<span>
								<strong>
									{{ author.name }}
								</strong>
							</span>
						</div>
					</div>
				</div>
			</Card>
		</div>
	</div>
</template>

<script setup>
import {
	CheckIcon,
	DownloadIcon,
	ExternalIcon,
	FileIcon,
	RefreshCwIcon,
	ReportIcon,
} from '@modrinth/assets'
import {
	Avatar,
	Breadcrumbs,
	Button,
	ButtonLink,
	Card,
	useFormatBytes,
	useFormatDateTime,
} from '@modrinth/ui'
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'

import { useBreadcrumb } from '@/providers/breadcrumbs'

const formatDateTime = useFormatDateTime({
	timeStyle: 'short',
	dateStyle: 'long',
})
const formatBytes = useFormatBytes()

const route = useRoute()
const router = useRouter()

const props = defineProps({
	project: {
		type: Object,
		required: true,
	},
	versions: {
		type: Array,
		required: true,
	},
	members: {
		type: Array,
		required: true,
	},
	install: {
		type: Function,
		required: true,
	},
	installed: {
		type: Boolean,
		required: true,
	},
	installing: {
		type: Boolean,
		required: true,
	},
	installedVersion: {
		type: String,
		required: true,
	},
})

const version = ref(props.versions.find((v) => v.id === Number(route.params.version)))

const versionBreadcrumbLabel = computed(() => version.value?.displayName || 'Version')
useBreadcrumb({
	slot: 'curseforge-version',
	id: () => `curseforge-version:${String(route.params.version ?? '')}`,
	label: versionBreadcrumbLabel,
	to: () => route.fullPath,
})

// CfFile.id — число, installedVersion приходит строкой, поэтому сравниваем через String()
const isInstalledVersion = computed(
	() => props.installed && String(version.value?.id) === props.installedVersion,
)

watch(
	() => props.versions,
	async () => {
		if (route.params.version) {
			version.value = props.versions.find((v) => v.id === Number(route.params.version))
		}
	},
)

const author = computed(() => (props.members ? props.members[0] : null))

const displayDependencies = ref([])
// Для CF зависимости — пока упрощённо, можно расширить позже
</script>

<style scoped lang="scss">
.version-container {
	display: flex;
	flex-direction: row;
	gap: 1rem;
}

.version-title {
	margin-bottom: 1rem;

	h2 {
		font-size: var(--font-size-2xl);
		font-weight: 700;
		color: var(--color-contrast);
		margin: 0;
	}
}

.dependency {
	display: flex;
	padding: 0.5rem 1rem 0.5rem 0.5rem;
	gap: 0.5rem;
	background: var(--color-raised-bg);
	color: var(--color-base);
	width: 100%;
	cursor: pointer;

	.title {
		font-weight: bolder;
	}
}

.file {
	display: flex;
	flex-direction: row;
	gap: 0.5rem;
	background: var(--color-button-bg);
	color: var(--color-base);
	padding: 0.5rem 1rem;

	.download {
		margin-left: auto;
		background-color: var(--color-raised-bg);
	}

	.label {
		display: flex;
		margin: auto 0;
		gap: 0.5rem;

		.title {
			font-weight: bolder;
			word-break: break-all;
		}

		svg {
			min-width: 1.1rem;
			min-height: 1.1rem;
			width: 1.1rem;
			height: 1.1rem;
			margin: auto 0;
		}
	}
}

.button-group {
	display: flex;
	flex-wrap: wrap;
	flex-direction: row;
	gap: 0.5rem;
}

.card-title {
	font-size: var(--font-size-lg);
	color: var(--color-contrast);
	margin: 0 0 0.5rem;
}

.description-cards {
	width: 100%;
}

.metadata-card {
	width: 20rem;
	height: min-content;
}

.metadata {
	display: flex;
	flex-direction: column;
	flex-wrap: wrap;
	gap: 1rem;

	.metadata-item {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;

		.metadata-label {
			font-weight: bold;
		}
	}
}

.author {
	display: flex;
	flex-direction: row;
	gap: 0.5rem;
	align-items: center;
	text-decoration: none;
	color: var(--color-base);
	background: var(--color-raised-bg);
	padding: 0.5rem;
	width: 100%;
	box-shadow: none;
}

.markdown-body {
	:deep(hr),
	:deep(h1),
	:deep(h2),
	img {
		max-width: max(60rem, 90%) !important;
	}

	:deep(ul),
	:deep(ol) {
		margin-left: 2rem;
	}
}
</style>
