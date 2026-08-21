/**
 * All theseus API calls return serialized values (both return values and errors);
 * So, for example, addDefaultInstance creates a blank instance object, where the Rust struct is serialized,
 *  and deserialized into a usable JS object.
 */
import { invoke } from '@tauri-apps/api/core'

import type { FeatureFlag } from '@/composables/use-app-settings.ts'
import type { ColorTheme } from '@/composables/use-theme.ts'
import type { Hooks, MemorySettings, WindowSize } from '@/helpers/types'

// Settings object
/*

Settings {
    "memory": MemorySettings,
    "game_resolution": [int int],
    "custom_java_args": [String ...],
    "custom_env_args" : [(string, string) ... ]>,
    "java_globals": Hash of (string, Path),
    "default_user": Uuid string (can be null),
    "hooks": Hooks,
    "max_concurrent_downloads": uint,
    "version": u32,
    "collapsed_navigation": bool,
}

Memorysettings {
    "min": u32, can be null,
    "max": u32,
}

*/

export type AppSettings = {
	max_concurrent_downloads: number
	max_concurrent_writes: number

	theme: ColorTheme
	locale: string
	default_page: 'Home' | 'Library'
	collapsed_navigation: boolean
	hide_nametag_skins_page: boolean
	advanced_rendering: boolean
	native_decorations: boolean
	toggle_sidebar: boolean
	sync_theme_across_devices: boolean
	sync_behavior_across_devices: boolean

	telemetry: boolean
	discord_rpc: boolean
	personalized_ads: boolean

	extra_launch_args: string[]
	custom_env_vars: [string, string][]
	memory: MemorySettings
	force_fullscreen: boolean
	game_resolution: WindowSize
	hide_on_process_start: boolean
	hooks: Hooks

	custom_dir?: string | null
	prev_custom_dir?: string | null
	migrated: boolean

	developer_mode: boolean
	feature_flags: Record<FeatureFlag, boolean>

	skipped_update: string | null
	pending_update_toast_for_version: string | null
	auto_download_updates: boolean | null

	modlex_vk_token?: string | null
	modlex_vk_user_id?: number | null
	modlex_soundcloud_enabled: boolean
	modlex_local_music_path?: string | null
	modlex_music_default_source: 'vk' | 'soundcloud' | 'local'
	modlex_playlists: ModlexPlaylist[]

	modlex_update_channel: 'stable' | 'beta'
	modlex_tester_id?: string | null

	version: number
}

export type ModlexTrackSource = 'local' | 'soundcloud' | 'vk' | 'radio'

export type ModlexTrackRef = {
	source: ModlexTrackSource
	ref: string
	title: string
	artist: string
	duration: number
}

export type ModlexPlaylist = {
	name: string
	tracks: ModlexTrackRef[]
}

export type ModlexLocalMusicFile = {
	path: string
	file_name: string
}

export async function modlexListLocalMusicFiles(folder: string): Promise<ModlexLocalMusicFile[]> {
	return await invoke('plugin:settings|modlex_list_local_music_files', { folder })
}

export function serializeEnvVars(vars: [string, string][] | undefined | null): string {
	return (vars ?? []).map(([key, value]) => `${key}=${value}`).join(' ')
}

export function parseEnvVars(input: string | undefined | null): [string, string][] {
	if (!input?.trim()) {
		return []
	}

	const vars: [string, string][] = []
	for (const entry of input.trim().split(/\s+/)) {
		const separator = entry.indexOf('=')
		if (separator <= 0) {
			continue
		}
		vars.push([entry.slice(0, separator), entry.slice(separator + 1)])
	}
	return vars
}

// Get full settings object
export async function get() {
	return (await invoke('plugin:settings|settings_get')) as AppSettings
}

// Set full settings object
export async function set(settings: AppSettings) {
	return await invoke('plugin:settings|settings_set', { settings })
}

export async function cancel_directory_change(): Promise<void> {
	return await invoke('plugin:settings|cancel_directory_change')
}
