<script setup lang="ts">
import { XIcon } from '@modrinth/assets'
import {
	ButtonStyled,
	commonMessages,
	injectNotificationManager,
	SearchSidebarFilter,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import { cf_get_categories, type CfCategory } from '@/helpers/curseforge'
import { get_game_versions, get_loaders } from '@/helpers/tags'

const route = useRoute()
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const props = defineProps<{
	classId: number
	instance?: { game_version: string; loader: string } | null
}>()

const emit = defineEmits<{
	categorySelect: [categoryId: number | null]
	gameVersionChange: [version: string | null]
	loaderChange: [loader: string | null]
}>()

const categories = ref<CfCategory[]>([])
const gameVersions = ref<string[]>([])
const loaders = ref<string[]>([])
const selectedCategory = ref<number | null>(null)
const selectedGameVersion = ref<string>('')
const selectedLoader = ref<string>('')
const isInstanceLocked = ref(false)
const filtersMenuOpen = ref(false)

// filter.options[].formatted_name is what SearchSidebarFilter actually renders
// (falls back to raw .id — a numeric CF category id — if missing). The old
// version of this file used .name here instead, which is exactly why category
// filters used to show raw numbers instead of names.
const filterTypes = computed(() => [
	{
		id: 'cf_categories',
		formatted_name: 'Категории',
		display: 'list' as const,
		options: categories.value.map((cat) => ({
			id: String(cat.id),
			formatted_name: cat.name || 'Unknown',
			icon: undefined,
			value: String(cat.id),
			method: 'and' as const,
		})),
	},
	{
		id: 'game_version',
		formatted_name: 'Версия Minecraft',
		display: 'list' as const,
		options: gameVersions.value.map((version) => ({
			id: version || 'unknown',
			formatted_name: version || 'Unknown',
			icon: undefined,
			value: version,
			method: 'and' as const,
		})),
	},
	{
		id: 'mod_loader',
		formatted_name: 'Загрузчик',
		display: 'list' as const,
		options: (loaders.value || []).map((loader) => {
			const loaderName = loader || 'unknown'
			return {
				id: loaderName,
				formatted_name: loaderName.charAt(0).toUpperCase() + loaderName.slice(1),
				icon: undefined,
				value: loaderName,
				method: 'and' as const,
			}
		}),
	},
])

const providedFilters = computed(() => {
	const filters = []
	if (isInstanceLocked.value && props.instance) {
		filters.push({ type: 'game_version', option: props.instance.game_version })
		filters.push({ type: 'mod_loader', option: props.instance.loader })
	}
	return filters
})

const currentFilters = ref<Array<{ type: string; option: string; negative?: boolean }>>([])
const toggledGroups = ref<string[]>([])
const overriddenProvidedFilterTypes = ref<string[]>([])

async function loadData() {
	try {
		const [cats, gvs, lds] = await Promise.all([
			cf_get_categories(props.classId).catch(handleError),
			get_game_versions().catch(handleError),
			get_loaders().catch(handleError),
		])
		categories.value = cats || []
		gameVersions.value = (gvs ?? []).map((g) => g.version).filter((v): v is string => !!v)
		loaders.value = (lds ?? []).map((l) => l.loader).filter((v): v is string => !!v)
	} catch (e) {
		handleError(e)
	}
}

watch(() => props.classId, loadData)

onMounted(() => {
	loadData()
	isInstanceLocked.value = !!route.query.i
})

watch(
	() => route.query.i,
	(instanceId) => {
		isInstanceLocked.value = !!instanceId
	},
)

function onFilterChange() {
	const selectedCategoryFilter = currentFilters.value.find((f) => f.type === 'cf_categories')
	if (selectedCategoryFilter) {
		const catId = Number(selectedCategoryFilter.option)
		if (!Number.isNaN(catId)) {
			selectedCategory.value = catId
			emit('categorySelect', catId)
		}
	} else {
		selectedCategory.value = null
		emit('categorySelect', null)
	}

	const gameVersionFilter = currentFilters.value.find((f) => f.type === 'game_version')
	if (gameVersionFilter) {
		selectedGameVersion.value = gameVersionFilter.option
		emit('gameVersionChange', gameVersionFilter.option)
	} else {
		selectedGameVersion.value = ''
		emit('gameVersionChange', null)
	}

	const loaderFilter = currentFilters.value.find((f) => f.type === 'mod_loader')
	if (loaderFilter) {
		selectedLoader.value = loaderFilter.option
		emit('loaderChange', loaderFilter.option)
	} else {
		selectedLoader.value = ''
		emit('loaderChange', null)
	}
}

const filterClass = computed(
	() =>
		'border-0 border-b-[1px] [&:first-child>button]:pt-4 last:border-b-0 border-[--cf-gradient-border] border-solid',
)
const buttonClass = computed(
	() =>
		'button-animation flex flex-col gap-1 px-3 py-3 w-full bg-transparent cursor-pointer border-none hover:bg-button-bg',
)
const contentClass = computed(() => 'mt-2 mb-3')
const innerPanelClass = computed(() => 'ml-2 mr-3')

function closeFiltersMenu() {
	if (filtersMenuOpen.value) {
		filtersMenuOpen.value = false
	}
	window.scrollTo({ top: 0, behavior: 'instant' as ScrollBehavior })
}

const lockedMessages = computed(() => ({
	gameVersion: isInstanceLocked.value ? 'Управляется сборкой' : undefined,
	modLoader: isInstanceLocked.value ? 'Управляется сборкой' : undefined,
	providedBy: isInstanceLocked.value ? 'Управляется сборкой' : undefined,
	syncButton: 'Синхронизировать',
}))

const filterOptions = computed(() => filterTypes.value.filter((f) => f.options.length > 0))
</script>

<template>
	<div class="cf-sidebar-wrapper">
		<div v-if="filtersMenuOpen" class="fixed inset-0 z-40 bg-bg" />

		<div
			class="flex flex-col"
			:class="{
				'fixed inset-0 z-50 m-4 mb-0 overflow-auto rounded-t-3xl bg-bg-raised': filtersMenuOpen,
			}"
		>
			<div
				v-if="filtersMenuOpen"
				class="sticky top-0 z-10 mx-1 flex items-center justify-between gap-3 border-0 border-b-[1px] border-solid border-divider bg-bg-raised px-6 py-4"
			>
				<h3 class="m-0 text-lg text-contrast">{{ formatMessage(commonMessages.filtersLabel) }}</h3>
				<ButtonStyled circular>
					<button @click="closeFiltersMenu">
						<XIcon />
					</button>
				</ButtonStyled>
			</div>

			<template v-for="filterType in filterOptions" :key="`cf-filter-${filterType.id}`">
				<SearchSidebarFilter
					v-model:selected-filters="currentFilters"
					v-model:toggled-groups="toggledGroups"
					v-model:overridden-provided-filter-types="overriddenProvidedFilterTypes"
					:provided-filters="providedFilters"
					:filter-type="filterType"
					:class="filterClass"
					:button-class="buttonClass"
					:content-class="contentClass"
					:inner-panel-class="innerPanelClass"
					:open-by-default="true"
					@update:selected-filters="onFilterChange"
				>
					<template #header>
						<h3 class="text-base m-0 text-contrast">
							{{ filterType.formatted_name }}
						</h3>
					</template>
					<template
						v-if="lockedMessages?.gameVersion && filterType.id === 'game_version'"
						#locked-game_version
					>
						{{ lockedMessages.gameVersion }}
					</template>
					<template
						v-if="lockedMessages?.modLoader && filterType.id === 'mod_loader'"
						#locked-mod_loader
					>
						{{ lockedMessages.modLoader }}
					</template>
					<template v-if="lockedMessages?.syncButton" #sync-button>
						{{ lockedMessages.syncButton }}
					</template>
				</SearchSidebarFilter>
			</template>
		</div>
	</div>
</template>

<style scoped lang="scss">
.cf-sidebar-wrapper {
	--cf-gradient-border: rgba(241, 100, 54, 0.2);

	:deep(.SearchSidebarFilter) {
		.filter-header {
			border-color: var(--cf-gradient-border);
		}

		.filter-option {
			&:hover {
				background: rgba(241, 100, 54, 0.08);
			}

			&.active {
				background: rgba(241, 100, 54, 0.15);
				color: #f16436;
			}
		}

		.filter-checkbox {
			&:checked {
				accent-color: #f16436;
			}
		}
	}
}
</style>
