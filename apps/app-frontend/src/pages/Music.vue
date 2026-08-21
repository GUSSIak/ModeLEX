<template>
	<div class="music-page box-border min-h-full p-4">
		<h1 class="m-0 mb-4 text-2xl font-bold flex items-center gap-2">
			<AudioIcon />
			Музыка
		</h1>
		<p class="m-0 mb-6 text-secondary">
			ВНИМАНИЕ ЕСЛИ ВЫ ПО КАКОЙ ТО ПРИЧИНЕ УВИДЕЛИ ЭТУ ВКЛАДКУ ОТКЛЮЧИТЕ ЕЕ В НАСТРОЙКАХ И СДЕЛАЙТЕ
			ВИД ЧТО ЕЕ НЕ ВИДЕЛИ, ПРИ ПОПЫТКЕ ЧТО ТО ТУТ ИЗМЕНИТЬ ВЫ МОЖЕТЕ СЛОМАТЬ ИГРУ И САМ ЛАУНЧЕР
			ДАННАЯ ФУНКЦИЯ ТОЛЬКО В РАЗРАБОТКЕ И ТРЕБУЕТ ДЛЯ РАБОТЫ СПЕЦИАЛИЗИРОВАННОЙ ДЕВ ВЕРСИИ ЛАУНЧЕРА
			Настройки встроенного музыкального плеера мода ModLEX Core — играет прямо в игре. Здесь
			настраиваются источники и составляются плейлисты;
		</p>

		<div class="settings-section">
			<h2 class="settings-section__title">Источники музыки</h2>

			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">SoundCloud</h3>
					<p class="setting-row__desc">Работает у всех сразу, вход в аккаунт не нужен.</p>
				</div>
				<Toggle v-model="settings.modlex_soundcloud_enabled" />
			</div>

			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Локальная папка с музыкой</h3>
					<p class="setting-row__desc">Мод будет проигрывать аудиофайлы из выбранной папки.</p>
				</div>
				<ButtonStyled>
					<button @click="pickLocalMusicFolder">
						<FolderOpenIcon />
						{{ settings.modlex_local_music_path ? 'Изменить' : 'Выбрать папку' }}
					</button>
				</ButtonStyled>
			</div>
			<p v-if="settings.modlex_local_music_path" class="setting-row__path">
				{{ settings.modlex_local_music_path }}
			</p>

			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Источник по умолчанию</h3>
					<p class="setting-row__desc">Какой источник плеер мода будет использовать при старте.</p>
				</div>
				<DropdownSelect
					v-model="settings.modlex_music_default_source"
					name="music-default-source"
					:options="sourceOptions"
					:get-option-label="getSourceLabel"
					class="settings-dropdown"
				/>
			</div>
		</div>

		<div class="settings-section">
			<h2 class="settings-section__title">VK Music — для продвинутых пользователей</h2>
			<p class="settings-section__desc settings-section__warning">
				⚠ Лаунчер никогда не запрашивает логин и пароль от VK и не входит в аккаунт за тебя. Чтобы
				использовать VK как источник, получи access-токен самостоятельно (например, локальным
				open-source инструментом на своей машине) и вставь его ниже. Токен хранится только локально
				на этом устройстве и отправляется только на api.vk.com — никогда на сервера ModLEX (их и
				нет). Доступ к аудио VK через сторонний токен нарушает правила VK и может привести к
				блокировке твоего аккаунта. Используешь на свой риск.
			</p>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">VK-токен</h3>
				</div>
			</div>
			<StyledInput
				id="modlex-vk-token"
				v-model="vkToken"
				type="password"
				placeholder="Вставь свой access-токен VK"
			/>
		</div>

		<div class="settings-section">
			<h2 class="settings-section__title">Плейлисты</h2>
			<p class="settings-section__desc">
				Собери плейлисты из локальных файлов и прямых радио-ссылок — мод подхватит их как стартовые
				очереди. Добавление из SoundCloud и VK появится, когда будут готовы резолверы на стороне
				мода.
			</p>

			<div class="playlist-layout">
				<div class="playlist-sidebar">
					<div class="playlist-list">
						<button
							v-for="(playlist, index) in settings.modlex_playlists"
							:key="index"
							type="button"
							class="playlist-list__item"
							:class="{ 'playlist-list__item--active': index === selectedPlaylistIndex }"
							@click="selectedPlaylistIndex = index"
						>
							<span class="playlist-list__name">{{ playlist.name || 'Без названия' }}</span>
							<span class="playlist-list__count">{{ playlist.tracks.length }}</span>
						</button>
						<p v-if="settings.modlex_playlists.length === 0" class="playlist-empty">
							Плейлистов пока нет.
						</p>
					</div>
					<div class="playlist-create">
						<StyledInput
							id="new-playlist-name"
							v-model="newPlaylistName"
							placeholder="Название плейлиста"
							@keydown.enter="createPlaylist"
						/>
						<ButtonStyled color="brand">
							<button :disabled="!newPlaylistName.trim()" @click="createPlaylist">
								<PlusIcon />
								Создать
							</button>
						</ButtonStyled>
					</div>
				</div>

				<div v-if="selectedPlaylist" class="playlist-editor">
					<div class="playlist-editor__header">
						<StyledInput
							id="playlist-name"
							v-model="selectedPlaylist.name"
							class="playlist-editor__name"
						/>
						<ButtonStyled circular>
							<button v-tooltip="'Дублировать'" @click="duplicatePlaylist(selectedPlaylistIndex!)">
								<CopyIcon />
							</button>
						</ButtonStyled>
						<ButtonStyled circular color="red">
							<button v-tooltip="'Удалить'" @click="deletePlaylist(selectedPlaylistIndex!)">
								<TrashIcon />
							</button>
						</ButtonStyled>
					</div>

					<Draggable
						:list="selectedPlaylist.tracks"
						item-key="ref"
						class="playlist-tracks"
						handle=".track-drag-handle"
						:animation="200"
					>
						<template #item="{ element: track, index: trackIndex }">
							<div class="playlist-track">
								<GripVerticalIcon class="track-drag-handle" />
								<span class="playlist-track__badge">{{ track.source }}</span>
								<div class="playlist-track__info">
									<span class="playlist-track__title">{{ track.title || track.ref }}</span>
									<span v-if="track.artist" class="playlist-track__artist">{{ track.artist }}</span>
								</div>
								<button
									type="button"
									class="playlist-track__remove"
									@click="removeTrack(trackIndex)"
								>
									<XIcon />
								</button>
							</div>
						</template>
					</Draggable>
					<p v-if="selectedPlaylist.tracks.length === 0" class="playlist-empty">
						Треков пока нет — добавь снизу.
					</p>

					<div class="playlist-add">
						<h3 class="playlist-add__title">Добавить из локальной папки</h3>
						<p v-if="!settings.modlex_local_music_path" class="setting-row__desc">
							Сначала выбери папку с музыкой выше.
						</p>
						<div v-else class="playlist-add__files">
							<button
								v-for="file in localFiles"
								:key="file.path"
								type="button"
								class="playlist-add__file"
								@click="addLocalTrack(file)"
							>
								<PlusIcon class="size-4" />
								{{ file.file_name }}
							</button>
							<p v-if="localFiles.length === 0" class="playlist-empty">
								В папке не найдено аудиофайлов.
							</p>
						</div>

						<h3 class="playlist-add__title">Добавить по прямой ссылке (радио)</h3>
						<div class="playlist-add__radio">
							<StyledInput
								id="radio-url"
								v-model="radioUrl"
								placeholder="https://... прямой поток mp3/aac"
							/>
							<ButtonStyled>
								<button :disabled="!radioUrl.trim()" @click="addRadioTrack">
									<PlusIcon />
									Добавить
								</button>
							</ButtonStyled>
						</div>
					</div>
				</div>
				<p v-else class="playlist-empty playlist-empty--placeholder">
					Выбери плейлист слева или создай новый.
				</p>
			</div>
		</div>
	</div>
</template>

<script setup lang="ts">
import {
	CopyIcon,
	FolderOpenIcon,
	GripVerticalIcon,
	PlusIcon,
	TagCategoryAudioIcon as AudioIcon,
	TrashIcon,
	XIcon,
} from '@modrinth/assets'
import { ButtonStyled, DropdownSelect, StyledInput, Toggle } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, ref, watch } from 'vue'
import Draggable from 'vuedraggable'

import {
	get,
	modlexListLocalMusicFiles,
	type ModlexLocalMusicFile,
	set,
} from '@/helpers/settings.ts'
import { useRootBreadcrumb } from '@/providers/breadcrumbs'

useRootBreadcrumb({
	slot: 'root',
	id: 'music',
	label: 'Музыка',
	to: '/music',
	visual: { type: 'icon', component: AudioIcon },
})

type MusicSource = 'vk' | 'soundcloud' | 'local'

const sourceOptions: MusicSource[] = ['soundcloud', 'local', 'vk']

function getSourceLabel(value: MusicSource): string {
	return { vk: 'VK Music', soundcloud: 'SoundCloud', local: 'Локальная папка' }[value] ?? value
}

const settings = ref(await get())

const vkToken = computed({
	get: () => settings.value.modlex_vk_token ?? '',
	set: (value: string) => {
		settings.value.modlex_vk_token = value.length > 0 ? value : null
	},
})

watch(
	settings,
	async () => {
		await set(settings.value)
	},
	{ deep: true },
)

async function pickLocalMusicFolder() {
	const newDir = await open({
		multiple: false,
		directory: true,
		title: 'Выбери папку с музыкой',
	})

	if (newDir) {
		settings.value.modlex_local_music_path = newDir
	}
}

const selectedPlaylistIndex = ref<number | null>(null)
const selectedPlaylist = computed(() =>
	selectedPlaylistIndex.value !== null
		? (settings.value.modlex_playlists[selectedPlaylistIndex.value] ?? null)
		: null,
)

const newPlaylistName = ref('')
const radioUrl = ref('')
const localFiles = ref<ModlexLocalMusicFile[]>([])

function createPlaylist() {
	const name = newPlaylistName.value.trim()
	if (!name) return
	settings.value.modlex_playlists.push({ name, tracks: [] })
	selectedPlaylistIndex.value = settings.value.modlex_playlists.length - 1
	newPlaylistName.value = ''
}

function duplicatePlaylist(index: number) {
	const source = settings.value.modlex_playlists[index]
	if (!source) return
	settings.value.modlex_playlists.splice(index + 1, 0, {
		name: `${source.name} (копия)`,
		tracks: source.tracks.map((track) => ({ ...track })),
	})
	selectedPlaylistIndex.value = index + 1
}

function deletePlaylist(index: number) {
	settings.value.modlex_playlists.splice(index, 1)
	if (selectedPlaylistIndex.value === index) {
		selectedPlaylistIndex.value = null
	} else if (selectedPlaylistIndex.value !== null && selectedPlaylistIndex.value > index) {
		selectedPlaylistIndex.value -= 1
	}
}

function removeTrack(trackIndex: number) {
	selectedPlaylist.value?.tracks.splice(trackIndex, 1)
}

function addLocalTrack(file: ModlexLocalMusicFile) {
	if (!selectedPlaylist.value) return
	selectedPlaylist.value.tracks.push({
		source: 'local',
		ref: file.path,
		title: file.file_name.replace(/\.[^.]+$/, ''),
		artist: '',
		duration: 0,
	})
}

function addRadioTrack() {
	const url = radioUrl.value.trim()
	if (!url || !selectedPlaylist.value) return
	selectedPlaylist.value.tracks.push({
		source: 'radio',
		ref: url,
		title: url,
		artist: '',
		duration: 0,
	})
	radioUrl.value = ''
}

watch(
	() => settings.value.modlex_local_music_path,
	async (folder) => {
		localFiles.value = folder ? await modlexListLocalMusicFiles(folder).catch(() => []) : []
	},
	{ immediate: true },
)
</script>

<style scoped>
.music-page {
	max-width: 60rem;
}

.settings-section {
	padding-bottom: 1.25rem;
	margin-bottom: 1.25rem;
	border-bottom: 1px solid var(--color-divider);
}

.settings-section:last-child {
	border-bottom: none;
	margin-bottom: 0;
	padding-bottom: 0;
}

.settings-section__title {
	font-size: 1.1rem;
	font-weight: 600;
	margin-bottom: 0.75rem;
	color: var(--color-contrast);
}

.settings-section__desc {
	margin: 0 0 1rem;
	font-size: 0.875rem;
	color: var(--color-secondary);
}

.settings-section__warning {
	color: var(--color-orange);
}

.setting-row {
	display: flex;
	align-items: center;
	gap: 1rem;
	padding: 0.5rem 0;
}

.setting-row__info {
	flex: 1;
	min-width: 0;
}

.setting-row__label {
	margin: 0 0 0.2rem;
	font-size: 1rem;
	font-weight: 700;
	color: var(--color-contrast);
}

.setting-row__desc {
	margin: 0;
	font-size: 0.875rem;
	color: var(--color-secondary);
}

.setting-row__path {
	margin: -0.25rem 0 0.5rem;
	font-size: 0.8rem;
	color: var(--color-secondary);
	word-break: break-all;
}

.settings-dropdown {
	flex-shrink: 0;
	min-width: 160px;
}

.playlist-layout {
	display: grid;
	grid-template-columns: minmax(180px, 240px) 1fr;
	gap: 1rem;
	align-items: start;
}

.playlist-sidebar {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
}

.playlist-list {
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
	max-height: 20rem;
	overflow-y: auto;
}

.playlist-list__item {
	display: flex;
	align-items: center;
	justify-content: space-between;
	gap: 0.5rem;
	padding: 0.5rem 0.75rem;
	border: 1px solid var(--color-divider);
	border-radius: 0.5rem;
	background: var(--color-button-bg);
	color: var(--color-contrast);
	cursor: pointer;
	text-align: left;
	font: inherit;
}

.playlist-list__item--active {
	border-color: var(--color-brand);
	background: var(--color-brand-highlight);
}

.playlist-list__name {
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
}

.playlist-list__count {
	flex-shrink: 0;
	font-size: 0.75rem;
	color: var(--color-secondary);
}

.playlist-create {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
}

.playlist-editor {
	display: flex;
	flex-direction: column;
	gap: 0.75rem;
	min-width: 0;
}

.playlist-editor__header {
	display: flex;
	align-items: center;
	gap: 0.5rem;
}

.playlist-editor__name {
	flex: 1;
}

.playlist-tracks {
	display: flex;
	flex-direction: column;
	gap: 0.35rem;
	max-height: 20rem;
	overflow-y: auto;
}

.playlist-track {
	display: flex;
	align-items: center;
	gap: 0.6rem;
	padding: 0.4rem 0.6rem;
	border: 1px solid var(--color-divider);
	border-radius: 0.5rem;
	background: var(--color-button-bg);
}

.track-drag-handle {
	flex-shrink: 0;
	cursor: grab;
	color: var(--color-secondary);
}

.playlist-track__badge {
	flex-shrink: 0;
	font-size: 0.7rem;
	text-transform: uppercase;
	color: var(--color-secondary);
	background: var(--color-divider);
	border-radius: 0.25rem;
	padding: 0.1rem 0.35rem;
}

.playlist-track__info {
	flex: 1;
	min-width: 0;
	display: flex;
	flex-direction: column;
}

.playlist-track__title {
	overflow: hidden;
	text-overflow: ellipsis;
	white-space: nowrap;
	color: var(--color-contrast);
}

.playlist-track__artist {
	font-size: 0.75rem;
	color: var(--color-secondary);
}

.playlist-track__remove {
	flex-shrink: 0;
	background: transparent;
	border: none;
	color: var(--color-secondary);
	cursor: pointer;
	display: flex;
	align-items: center;
}

.playlist-add {
	display: flex;
	flex-direction: column;
	gap: 0.5rem;
}

.playlist-add__title {
	margin: 0.25rem 0 0;
	font-size: 0.85rem;
	font-weight: 600;
	color: var(--color-contrast);
}

.playlist-add__files {
	display: flex;
	flex-direction: column;
	gap: 0.25rem;
	max-height: 10rem;
	overflow-y: auto;
}

.playlist-add__file {
	display: flex;
	align-items: center;
	gap: 0.5rem;
	padding: 0.35rem 0.5rem;
	border: 1px dashed var(--color-divider);
	border-radius: 0.4rem;
	background: transparent;
	color: var(--color-contrast);
	cursor: pointer;
	text-align: left;
	font: inherit;
	font-size: 0.85rem;
}

.playlist-add__radio {
	display: flex;
	gap: 0.5rem;
}

.playlist-add__radio :deep(input) {
	flex: 1;
}

.playlist-empty {
	margin: 0.25rem 0;
	font-size: 0.8rem;
	color: var(--color-secondary);
}

.playlist-empty--placeholder {
	padding: 1rem;
	text-align: center;
	border: 1px dashed var(--color-divider);
	border-radius: 0.5rem;
}
</style>
