// apps/app-frontend/src/helpers/curseforge.ts
import type { ContentItem } from '@modrinth/ui'
import { invoke } from '@tauri-apps/api/core'

export interface CfLogo {
	url: string
	thumbnailUrl?: string
}

export interface CfAuthor {
	id: number
	name: string
}

export interface CfCategory {
	id: number
	name: string
	classId?: number
	parentCategoryId?: number
}

export interface CfMod {
	id: number
	name: string
	slug: string
	summary: string
	logo?: CfLogo
	downloadCount: number
	dateModified: string
	categories: CfCategory[]
	authors: CfAuthor[]
	links?: { websiteUrl?: string }
	classId?: number
}

export interface CfFileHash {
	value: string
	algo: number
}

export interface CfDependency {
	modId: number
	relationType: number
}

export interface CfFile {
	id: number
	modId: number
	displayName: string
	fileName: string
	downloadUrl?: string
	fileLength: number
	gameVersions: string[]
	dependencies: CfDependency[]
	fileDate: string
	hashes: CfFileHash[]
	releaseType: number
}

export interface CfSearchResult {
	data: CfMod[]
	pagination: {
		index: number
		pageSize: number
		resultCount: number
		totalCount: number
	}
}

export interface CfModDescription {
	html: string
}

export interface CfScreenshot {
	id: number
	thumbnailUrl: string
	url: string
	title: string
}

export interface CfModDetails extends CfMod {
	screenshots: CfScreenshot[]
	description: string
	links: {
		websiteUrl?: string
		sourceUrl?: string
	}
}

export interface CfInstallResult {
	profilePath: string
	installedIds: number[]
}

const CF_API_BASE = 'plugin:curseforge'

export async function cf_search(
	query: string,
	gameVersion?: string,
	loader?: string,
	classId = 6,
	sortField: 'popularity' | 'totalDownloads' | 'newest' | 'name' | 'rating' = 'popularity',
	sortOrder: 'asc' | 'desc' = 'desc',
	page = 0,
	categoryId?: number | null,
): Promise<CfSearchResult> {
	return await invoke(`${CF_API_BASE}|cf_search`, {
		query,
		gameVersion: gameVersion ?? null,
		loader: loader ?? null,
		classId,
		sortField,
		sortOrder,
		page,
		categoryId: categoryId ?? null,
	})
}

export async function cf_get_mod(modId: number): Promise<CfMod> {
	return await invoke(`${CF_API_BASE}|cf_get_mod`, { modId })
}

export async function cf_get_mod_details(modId: number): Promise<CfModDetails> {
	return await invoke(`${CF_API_BASE}|cf_get_mod_details`, { modId })
}

export async function cf_get_files(
	modId: number,
	gameVersion?: string,
	loader?: string,
): Promise<CfFile[]> {
	return await invoke(`${CF_API_BASE}|cf_get_files`, {
		modId,
		gameVersion: gameVersion ?? null,
		loader: loader ?? null,
	})
}

export async function cf_install_mod(
	profilePath: string,
	modId: number,
	fileId: number | null,
	gameVersion: string,
	loader: string,
): Promise<number[]> {
	return await invoke(`${CF_API_BASE}|cf_install_mod`, {
		profilePath,
		modId,
		fileId,
		gameVersion,
		loader,
	})
}

export async function cf_remove_mod(profilePath: string, modId: number): Promise<void> {
	return await invoke(`${CF_API_BASE}|cf_remove_mod`, { profilePath, modId })
}

export async function cf_get_description(modId: number): Promise<string> {
	return await invoke(`${CF_API_BASE}|cf_get_description`, { modId })
}

export async function cf_install_modpack(
	modId: number,
	fileId: number | null,
	gameVersion: string,
	loader: string,
): Promise<string> {
	return await invoke(`${CF_API_BASE}|cf_install_modpack`, {
		modId,
		fileId,
		gameVersion,
		loader,
	})
}

export async function cf_get_categories(classId?: number): Promise<CfCategory[]> {
	return await invoke(`${CF_API_BASE}|cf_get_categories`, { classId })
}

export async function cf_clear_cache(): Promise<void> {
	return await invoke(`${CF_API_BASE}|cf_clear_cache`)
}

export const CF_ICON_URL = 'https://www.curseforge.com/favicon.ico'
export const CF_COLOR = '#F16436'

export const CF_CLASS_IDS: Record<string, number> = {
	mod: 6,
	modpack: 4471,
	resourcepack: 12,
	shader: 6552,
	datapack: 6945,
	world: 17,
}

/** Maps a CurseForge classId back to a project type, defaulting to 'mod'. */
export function cfClassIdToProjectType(classId: number | undefined): string {
	const entry = Object.entries(CF_CLASS_IDS).find(([, id]) => id === classId)
	return entry ? entry[0] : 'mod'
}

export function getCfColorScheme() {
	return {
		primary: '#F16436',
		primaryDark: '#d4532a',
		primaryLight: '#ff8a61',
		primaryBg: 'rgba(241, 100, 54, 0.1)',
	}
}

const LOADER_NAMES = ['forge', 'fabric', 'quilt', 'neoforge']

/** Derives a file's Minecraft version + loader from its gameVersions tag list. */
export function extractFileTarget(file: CfFile): { gameVersion: string; loader: string } {
	const gameVersion = file.gameVersions.find((v) => /^\d+\.\d+/.test(v)) ?? ''
	const loader =
		file.gameVersions.find((v) => LOADER_NAMES.includes(v.toLowerCase()))?.toLowerCase() ?? 'forge'
	return { gameVersion, loader }
}

/**
 * Picks the newest file for a mod/modpack and derives its game version + loader,
 * mirroring how a plain "install latest version" pick works on the Modrinth side.
 */
export function resolveLatestFile(
	files: CfFile[],
): { file: CfFile; gameVersion: string; loader: string } | null {
	if (files.length === 0) return null
	const sorted = [...files].sort(
		(a, b) => new Date(b.fileDate).getTime() - new Date(a.fileDate).getTime(),
	)
	const withLoader = sorted.find((f) =>
		f.gameVersions.some((v) => LOADER_NAMES.includes(v.toLowerCase())),
	)
	const file = withLoader ?? sorted[0]
	return { file, ...extractFileTarget(file) }
}

function normalizeToken(value: string): string {
	return value.toLowerCase().replace(/[^a-z0-9]/g, '')
}

/**
 * Cross-source "is this the same mod already installed via the other platform" heuristic.
 * There's no real Modrinth-id <-> CurseForge-id mapping, so this matches by normalized title.
 */
export function findInstalledCounterpart(
	name: string,
	contentItems: ContentItem[],
): ContentItem | undefined {
	const target = normalizeToken(name)
	if (!target) return undefined
	return contentItems.find((item) => item.project && normalizeToken(item.project.title) === target)
}

/**
 * Loose match between a CurseForge file's display name and an installed version's
 * version number, used to decide whether a specific CF file is "the same version"
 * as an already-installed counterpart from the other platform.
 */
export function isSameCfFileVersion(cfFile: CfFile, versionNumber: string | undefined): boolean {
	if (!versionNumber) return false
	const a = normalizeToken(cfFile.displayName)
	const b = normalizeToken(versionNumber)
	if (!a || !b) return false
	return a.includes(b) || b.includes(a)
}
