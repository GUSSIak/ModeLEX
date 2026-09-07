// MODLEX: CF-аналог providers/content-install.ts — тот же переиспользуемый
// ContentInstallModal (он не завязан на источник данных, чистые пропсы/эвенты),
// но своя, независимая логика внутри, без единой правки в Modrinth-версии.
// Модрintх-путь (install() там) не подходит как есть: он резолвит через
// get_project(projectId) по Modrinth API, а у CF-мода нет Modrinth project_id.
import type { ContentInstallInstance, ContentInstallProjectInfo } from '@modrinth/ui'
import { createContext } from '@modrinth/ui'
import { ref } from 'vue'
import type { Router } from 'vue-router'

import { trackEvent } from '@/helpers/analytics'
import {
	CF_CLASS_IDS,
	type CfFile,
	type CfMod,
	cf_install_mod,
	extractFileTarget,
	findInstalledCounterpart,
	resolveLatestFile,
} from '@/helpers/curseforge'
import { install_create_instance, installJobInstanceId } from '@/helpers/install'
import {
	get as getInstance,
	get_content_items as getContentItems,
	getInstanceIconUrl,
	list as listInstances,
} from '@/helpers/instance'
import type { GameInstance, InstanceLoader } from '@/helpers/types'
import { get_instance_worlds, isSingleplayerWorld } from '@/helpers/worlds'

interface ModalRef {
	show: () => void
	hide: () => void
}

export interface CfContentInstallContext {
	instances: import('vue').Ref<ContentInstallInstance[]>
	compatibleLoaders: import('vue').Ref<string[]>
	gameVersions: import('vue').Ref<string[]>
	loading: import('vue').Ref<boolean>
	defaultTab: import('vue').Ref<'existing' | 'new'>
	projectInfo: import('vue').Ref<ContentInstallProjectInfo | null>
	handleInstallToInstance: (instance: ContentInstallInstance) => Promise<void>
	handleCreateAndInstall: (data: {
		name: string
		iconPath: string | null
		iconPreviewUrl: string | null
		loader: string
		gameVersion: string
	}) => Promise<void>
	handleNavigate: (instance: ContentInstallInstance) => void
	handleCancel: () => void
	setModal: (ref: ModalRef) => void
	/** Открывает модалку выбора/создания инстанции для установки CF-мода. */
	install: (mod: CfMod, files: CfFile[]) => Promise<void>
}

export const [injectCfContentInstall, provideCfContentInstall] =
	createContext<CfContentInstallContext>('root', 'cfContentInstall')

function cfLogoUrl(mod: CfMod): string | null {
	const thumb = mod.logo?.thumbnailUrl
	if (thumb) return thumb
	const full = mod.logo?.url
	return full || null
}

// MODLEX: resourcepacks/shaders/datapacks/worlds have no mod-loader concept at
// all — CurseForge just doesn't tag their files with a loader, so
// extractFileTarget() was falling back to its 'forge' default for every such
// file, and isCompatible()/pickFileFor() then compared that FABRICATED loader
// against each instance's real one. Result: every non-Forge instance got
// flagged "incompatible" for a project type where loader compatibility isn't
// even a meaningful question. Loader matching only makes sense for mod(6)/
// modpack(4471) — skip it entirely for the rest.
const LOADER_AGNOSTIC_CLASS_IDS = new Set<number>([
	CF_CLASS_IDS.resourcepack,
	CF_CLASS_IDS.shader,
	CF_CLASS_IDS.datapack,
	CF_CLASS_IDS.world,
])

function isLoaderAgnostic(classId: number | undefined): boolean {
	return classId !== undefined && LOADER_AGNOSTIC_CLASS_IDS.has(classId)
}

function targetsOf(files: CfFile[], classId: number | undefined) {
	const loaderAgnostic = isLoaderAgnostic(classId)
	const loaders = new Set<string>()
	const versions = new Set<string>()
	for (const file of files) {
		const target = extractFileTarget(file)
		if (!loaderAgnostic && target.loader) loaders.add(target.loader)
		if (target.gameVersion) versions.add(target.gameVersion)
	}
	return { loaders: [...loaders], versions: [...versions] }
}

function isCompatible(instance: GameInstance, files: CfFile[], classId: number | undefined): boolean {
	if (isLoaderAgnostic(classId)) return true
	return files.some((file) => {
		const target = extractFileTarget(file)
		return target.loader === instance.loader && target.gameVersion === instance.game_version
	})
}

function pickFileFor(
	files: CfFile[],
	loader: string,
	gameVersion: string,
	classId: number | undefined,
): CfFile | null {
	const loaderAgnostic = isLoaderAgnostic(classId)
	const matching = files.filter((file) => {
		const target = extractFileTarget(file)
		return (loaderAgnostic || target.loader === loader) && target.gameVersion === gameVersion
	})
	if (matching.length === 0) return resolveLatestFile(files)?.file ?? null
	return matching.sort((a, b) => new Date(b.fileDate).getTime() - new Date(a.fileDate).getTime())[0]
}

/** Datapacks belong to one specific world, not the instance as a whole —
 * resolves which one to target before install. Auto-picks when there's
 * exactly one; with several, falls back to whichever was played most
 * recently (no dedicated world-picker UI yet, but this lands right in the
 * common case rather than making datapack install impossible). Throws with a
 * user-facing message when there's nothing to install into at all. */
export async function resolveWorldFolder(instanceId: string): Promise<string> {
	const worlds = await get_instance_worlds(instanceId)
	const singleplayer = worlds.filter(isSingleplayerWorld)
	if (singleplayer.length === 0) {
		throw new Error(
			'В этой инстанции пока нет ни одного мира — сначала создайте мир, чтобы установить в него датапак.',
		)
	}
	if (singleplayer.length === 1) return singleplayer[0].path
	const sorted = [...singleplayer].sort((a, b) => {
		const at = a.last_played ? new Date(a.last_played).getTime() : 0
		const bt = b.last_played ? new Date(b.last_played).getTime() : 0
		return bt - at
	})
	return sorted[0].path
}

export function createCfContentInstall(opts: {
	router: Router
	handleError: (err: unknown) => void
}): CfContentInstallContext {
	const instancesList = ref<ContentInstallInstance[]>([])
	const compatibleLoaders = ref<string[]>([])
	const gameVersions = ref<string[]>([])
	const loading = ref(false)
	const defaultTab = ref<'existing' | 'new'>('existing')
	const projectInfo = ref<ContentInstallProjectInfo | null>(null)

	let modalRef: ModalRef | null = null
	let currentMod: CfMod | null = null
	let currentFiles: CfFile[] = []
	const installingIds = ref<Set<string>>(new Set())

	function applyInstalling() {
		instancesList.value = instancesList.value.map((instance) => ({
			...instance,
			installing: installingIds.value.has(instance.id),
		}))
	}

	async function install(mod: CfMod, files: CfFile[]) {
		currentMod = mod
		currentFiles = files
		installingIds.value = new Set()

		projectInfo.value = {
			title: mod.name,
			iconUrl: cfLogoUrl(mod),
			link: `/curseforge/${mod.id}`,
		}

		const targets = targetsOf(files, mod.classId)
		compatibleLoaders.value = targets.loaders
		gameVersions.value = targets.versions
		// Loader-agnostic types (resourcepacks etc.) never populate `loaders`
		// (see targetsOf), but "existing" is still the right default — there's
		// no loader requirement pushing the user toward creating a fresh instance.
		defaultTab.value =
			targets.loaders.length > 0 || isLoaderAgnostic(mod.classId) ? 'existing' : 'new'
		loading.value = true
		instancesList.value = []
		modalRef?.show()
		trackEvent('ProjectInstallStart', { source: 'CfProjectInstallModal' })

		try {
			const raw = await listInstances()
			const withInstalled = await Promise.all(
				raw.map(async (instance) => {
					const items = await getContentItems(instance.id).catch(() => [])
					const installed =
						items.some((item) => item.cf_mod_id === mod.id) ||
						!!findInstalledCounterpart(mod.name, items)
					return { instance, installed }
				}),
			)
			instancesList.value = withInstalled.map(({ instance, installed }) => ({
				id: instance.id,
				name: instance.name,
				iconUrl: getInstanceIconUrl(instance.icon_path),
				installed,
				compatible: isCompatible(instance, files, mod.classId),
				installing: false,
			}))
		} catch (err) {
			opts.handleError(err)
		} finally {
			loading.value = false
		}
	}

	async function handleInstallToInstance(instance: ContentInstallInstance) {
		if (!currentMod) return
		const full = await getInstance(instance.id).catch(() => null)
		if (!full) {
			opts.handleError('Instance not found')
			return
		}
		const file = pickFileFor(currentFiles, full.loader, full.game_version, currentMod.classId)
		if (!file) {
			opts.handleError('No CurseForge file available for this instance')
			return
		}

		let worldFolder: string | undefined
		if (currentMod.classId === CF_CLASS_IDS.datapack) {
			try {
				worldFolder = await resolveWorldFolder(instance.id)
			} catch (err) {
				opts.handleError(err)
				return
			}
		}

		installingIds.value = new Set(installingIds.value).add(instance.id)
		applyInstalling()
		try {
			await cf_install_mod(
				full.path,
				currentMod.id,
				file.id,
				full.game_version,
				full.loader,
				worldFolder,
			)
			instancesList.value = instancesList.value.map((item) =>
				item.id === instance.id ? { ...item, installed: true, installing: false } : item,
			)
			trackEvent('ProjectInstall', {
				loader: full.loader,
				game_version: full.game_version,
				id: String(currentMod.id),
				title: currentMod.name,
				source: 'CfProjectInstallModal',
			})
		} catch (err) {
			opts.handleError(err)
		} finally {
			installingIds.value = new Set([...installingIds.value].filter((id) => id !== instance.id))
			applyInstalling()
		}
	}

	async function handleCreateAndInstall(data: {
		name: string
		iconPath: string | null
		iconPreviewUrl: string | null
		loader: string
		gameVersion: string
	}) {
		if (!currentMod) return
		const file = pickFileFor(currentFiles, data.loader, data.gameVersion, currentMod.classId)
		if (!file) {
			opts.handleError('No CurseForge file available for this game version/loader')
			return
		}

		try {
			const job = await install_create_instance({
				name: data.name,
				gameVersion: data.gameVersion,
				loader: data.loader as InstanceLoader,
				loaderVersion: 'latest',
				iconPath: data.iconPath,
			})
			const id = installJobInstanceId(job)
			if (!id) return
			const full = await getInstance(id).catch(() => null)
			if (full) {
				await cf_install_mod(full.path, currentMod.id, file.id, data.gameVersion, data.loader)
			}
			trackEvent('InstanceCreate', { source: 'CfProjectInstallModal' })
			trackEvent('ProjectInstall', {
				loader: data.loader,
				game_version: data.gameVersion,
				id: String(currentMod.id),
				title: currentMod.name,
				source: 'CfProjectInstallModal',
			})
			await opts.router.push(`/instance/${encodeURIComponent(id)}`)
			modalRef?.hide()
		} catch (err) {
			opts.handleError(err)
		}
	}

	function handleNavigate(instance: ContentInstallInstance) {
		modalRef?.hide()
		opts.router.push(`/instance/${encodeURIComponent(instance.id)}`)
	}

	function handleCancel() {
		// Нечего откатывать — установка стартует по клику на конкретную
		// инстанцию, а не заранее, так что отмена просто закрывает модалку.
	}

	return {
		instances: instancesList,
		compatibleLoaders,
		gameVersions,
		loading,
		defaultTab,
		projectInfo,
		handleInstallToInstance,
		handleCreateAndInstall,
		handleNavigate,
		handleCancel,
		setModal(ref: ModalRef) {
			modalRef = ref
		},
		install,
	}
}
