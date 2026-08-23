<template>
	<div class="flex flex-col gap-6">
		<Transition name="settings-notice-fade">
			<div v-if="inlineNotice" class="settings-notice sticky top-2 z-20 mx-auto">
				{{ inlineNotice }}
			</div>
		</Transition>

		<!-- Внешний вид -->
		<div class="settings-section">
			<h2 class="settings-section__title">Внешний вид</h2>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Скрыть вкладку "Серверы"</h3>
					<p class="setting-row__desc">Убирает кнопку серверов из бокового меню.</p>
				</div>
				<Toggle v-model="modlexHideServers" />
			</div>
			<div v-if="musicFeatureEnabled" class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Скрыть вкладку "Музыка"</h3>
					<p class="setting-row__desc">
						Убирает кнопку музыкального плеера мода ModLEX Core из бокового меню.
					</p>
				</div>
				<Toggle v-model="modlexHideMusicTab" />
			</div>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Скрыть блок "Друзья"</h3>
					<p class="setting-row__desc">Убирает список друзей из правой панели.</p>
				</div>
				<Toggle v-model="modlexHideFriends" />
			</div>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Скрыть правую панель</h3>
					<p class="setting-row__desc">
						Панель (аккаунт, друзья, новости) полностью прячется, вместо неё — компактная плашка
						текущего аккаунта в углу. На странице модов панель вместо этого превращается в узкую
						полоску сбоку и выезжает целиком при наведении — там нужны фильтры категорий.
					</p>
				</div>
				<Toggle v-model="modlexHideRightSidebar" />
			</div>
			<div v-if="modlexHideRightSidebar" class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Прятать плашку аккаунта за край экрана</h3>
					<p class="setting-row__desc">
						Плашка тоже уезжает за правый край и выезжает обратно при наведении на угол экрана.
					</p>
				</div>
				<Toggle v-model="modlexHideFloatingAccountWidget" />
			</div>
			<div v-if="modlexHideRightSidebar" class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Эффект стекла</h3>
					<p class="setting-row__desc">
						Полупрозрачный фон с блюром вместо сплошного — для плашки аккаунта и
						полоски-подглядывания на Discover.
					</p>
				</div>
				<Toggle v-model="modlexFloatingGlassEffect" />
			</div>
		</div>

		<!-- Акцентный цвет -->
		<div class="settings-section">
			<h2 class="settings-section__title">Акцентный цвет</h2>
			<p class="settings-section__desc">
				Свой цвет вместо стандартного фиолетового — кнопки, ссылки, выделения.
			</p>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Цвет</h3>
					<p class="setting-row__desc">
						{{ modlexAccentColor ? modlexAccentColor : 'Стандартный цвет темы' }}
					</p>
				</div>
				<div class="flex items-center gap-2">
					<input
						type="color"
						class="color-picker"
						:value="accentColorForPicker"
						@input="modlexAccentColor = ($event.target as HTMLInputElement).value"
					/>
					<Button
						v-if="modlexAccentColor"
						type="outlined"
						size="sm"
						native-type="button"
						@click="modlexAccentColor = ''"
						>Сбросить</Button
					>
				</div>
			</div>

			<Button
				type="outlined"
				size="sm"
				native-type="button"
				class="!mt-3"
				@click="showAdvancedTheme = !showAdvancedTheme"
			>
				{{ showAdvancedTheme ? 'Скрыть расширенные настройки' : 'Расширенные настройки' }}
			</Button>

			<div v-if="showAdvancedTheme" class="advanced-theme">
				<p class="settings-section__desc">
					Тонкая настройка цветов интерфейса. Пустой цвет = стандартный цвет темы.
				</p>

				<div class="setting-row">
					<div class="setting-row__info">
						<h3 class="setting-row__label">Фон</h3>
						<p class="setting-row__desc">
							Основной фон контента —
							{{ modlexBgColor ? modlexBgColor : 'стандартный цвет темы' }}
						</p>
					</div>
					<div class="flex items-center gap-2">
						<input
							type="color"
							class="color-picker"
							:value="modlexBgColor || DEFAULT_BG"
							@input="modlexBgColor = ($event.target as HTMLInputElement).value"
						/>
						<Button
							v-if="modlexBgColor"
							type="outlined"
							size="sm"
							native-type="button"
							@click="modlexBgColor = ''"
							>Сбросить</Button
						>
					</div>
				</div>

				<div class="setting-row">
					<div class="setting-row__info">
						<h3 class="setting-row__label">Панели и карточки</h3>
						<p class="setting-row__desc">
							Карточки инстансов, кнопки, боковые панели, настройки —
							{{ modlexPanelColor ? modlexPanelColor : 'стандартный цвет темы' }}
						</p>
					</div>
					<div class="flex items-center gap-2">
						<input
							type="color"
							class="color-picker"
							:value="modlexPanelColor || DEFAULT_PANEL"
							@input="modlexPanelColor = ($event.target as HTMLInputElement).value"
						/>
						<Button
							v-if="modlexPanelColor"
							type="outlined"
							size="sm"
							native-type="button"
							@click="modlexPanelColor = ''"
							>Сбросить</Button
						>
					</div>
				</div>

				<div class="setting-row">
					<div class="setting-row__info">
						<h3 class="setting-row__label">Текст</h3>
						<p class="setting-row__desc">
							{{ modlexTextColor ? modlexTextColor : 'Стандартный цвет темы' }}
						</p>
					</div>
					<div class="flex items-center gap-2">
						<input
							type="color"
							class="color-picker"
							:value="modlexTextColor || DEFAULT_TEXT"
							@input="modlexTextColor = ($event.target as HTMLInputElement).value"
						/>
						<Button
							v-if="modlexTextColor"
							type="outlined"
							size="sm"
							native-type="button"
							@click="modlexTextColor = ''"
							>Сбросить</Button
						>
					</div>
				</div>

				<div class="setting-row">
					<div class="setting-row__info">
						<h3 class="setting-row__label">Обводка текста</h3>
						<p class="setting-row__desc">Контурная линия вокруг текста по всему приложению.</p>
					</div>
					<Toggle v-model="modlexTextOutlineEnabled" />
				</div>

				<div v-if="modlexTextOutlineEnabled" class="setting-row">
					<div class="setting-row__info">
						<h3 class="setting-row__label">Цвет обводки текста</h3>
					</div>
					<input v-model="modlexTextOutlineColor" type="color" class="color-picker" />
				</div>

				<div class="setting-row">
					<div class="setting-row__info">
						<h3 class="setting-row__label">Иконки бокового меню</h3>
						<p class="setting-row__desc">
							{{ modlexIconColor ? modlexIconColor : 'Стандартный цвет темы' }}
						</p>
					</div>
					<div class="flex items-center gap-2">
						<input
							type="color"
							class="color-picker"
							:value="modlexIconColor || DEFAULT_ICON"
							@input="modlexIconColor = ($event.target as HTMLInputElement).value"
						/>
						<Button
							v-if="modlexIconColor"
							type="outlined"
							size="sm"
							native-type="button"
							@click="modlexIconColor = ''"
							>Сбросить</Button
						>
					</div>
				</div>

				<div class="setting-row">
					<div class="setting-row__info">
						<h3 class="setting-row__label">Разделители и края</h3>
						<p class="setting-row__desc">
							{{ modlexDividerColor ? modlexDividerColor : 'Стандартный цвет темы' }}
						</p>
					</div>
					<div class="flex items-center gap-2">
						<input
							type="color"
							class="color-picker"
							:value="modlexDividerColor || DEFAULT_DIVIDER"
							@input="modlexDividerColor = ($event.target as HTMLInputElement).value"
						/>
						<Button
							v-if="modlexDividerColor"
							type="outlined"
							size="sm"
							native-type="button"
							@click="modlexDividerColor = ''"
							>Сбросить</Button
						>
					</div>
				</div>

				<div class="setting-row">
					<div class="setting-row__info">
						<h3 class="setting-row__label">Двойная обводка</h3>
						<p class="setting-row__desc">
							Доп. кольцо поверх краёв и разделителей — например, белая внутренняя и чёрная внешняя
							линия.
						</p>
					</div>
					<Toggle v-model="modlexDoubleBorderEnabled" />
				</div>

				<div v-if="modlexDoubleBorderEnabled" class="setting-row">
					<div class="setting-row__info">
						<h3 class="setting-row__label">Цвета обводки</h3>
						<p class="setting-row__desc">Внутренняя и внешняя линия.</p>
					</div>
					<div class="flex items-center gap-2">
						<input
							v-model="modlexDoubleBorderInner"
							v-tooltip="'Внутренняя линия'"
							type="color"
							class="color-picker"
						/>
						<input
							v-model="modlexDoubleBorderOuter"
							v-tooltip="'Внешняя линия'"
							type="color"
							class="color-picker"
						/>
					</div>
				</div>

				<div class="setting-row">
					<div class="setting-row__info">
						<h3 class="setting-row__label">Код темы</h3>
						<p class="setting-row__desc">Сохрани или перешли всю раскраску одной строкой.</p>
					</div>
					<Button type="outlined" size="sm" native-type="button" @click="copyThemeCode"
						>Скопировать код</Button
					>
				</div>

				<div class="setting-row">
					<input
						v-model="importThemeCodeInput"
						type="text"
						class="console-text-input flex-1"
						placeholder="Вставь код темы сюда"
						autocomplete="off"
						spellcheck="false"
					/>
					<Button type="outlined" size="sm" native-type="button" @click="applyThemeCodeInput"
						>Применить</Button
					>
				</div>
			</div>
		</div>

		<!-- Консоль запуска -->
		<div class="settings-section">
			<h2 class="settings-section__title">Консоль запуска</h2>
			<p class="settings-section__desc">
				Надпись на пустом экране консоли (пока нет запущенного процесса) и её размер.
			</p>
			<!-- ===== MODLEX: превью и ползунки размера/зазора временно отключены,
			     решение по UX предпросмотра ещё не принято — см. BaseTerminal.IDEAS.md =====
			<div class="console-preview">
				<BaseTerminal
					ref="previewTerminal"
					empty-state-type="instance"
					:empty-state-text="modlexConsoleText || undefined"
					:empty-state-scale="modlexConsoleScale || undefined"
					:empty-state-letter-gap="modlexConsoleLetterGap"
					:empty-state-fill-char="modlexConsoleFillChar || undefined"
					:empty-state-rain-chars="modlexConsoleRainChars || undefined"
					@ready="previewTerminal?.writeEmptyState()"
				/>
			</div>
			-->
			<div class="console-settings-controls">
				<label class="console-field">
					<span class="console-field__label">Надпись</span>
					<input
						v-model="modlexConsoleText"
						type="text"
						maxlength="20"
						placeholder="NO SIGNAL"
						class="console-text-input"
						autocomplete="off"
						autocorrect="off"
						spellcheck="false"
						data-1p-ignore
						data-lpignore="true"
					/>
				</label>
				<!--
				<label class="console-field">
					<span class="console-field__label">
						Размер {{ modlexConsoleScale > 0 ? `(${modlexConsoleScale})` : '(авто)' }}
					</span>
					<input v-model.number="modlexConsoleScale" type="range" min="0" max="6" step="1" />
				</label>
				<label class="console-field">
					<span class="console-field__label"
						>Зазор между буквами ({{ modlexConsoleLetterGap }})</span
					>
					<input v-model.number="modlexConsoleLetterGap" type="range" min="0" max="6" step="1" />
				</label>
				-->
				<label class="console-field">
					<span class="console-field__label">Символ, которым закрашены буквы</span>
					<input
						v-model="modlexConsoleFillChar"
						type="text"
						maxlength="1"
						placeholder="#"
						class="console-text-input console-text-input--narrow"
						autocomplete="off"
						spellcheck="false"
					/>
				</label>
				<label class="console-field">
					<span class="console-field__label">Алфавит символов дождя</span>
					<input
						v-model="modlexConsoleRainChars"
						type="text"
						placeholder="ｦｱｳｴｵｶｷｹｺｻｼｽｾｿﾀﾂﾃﾅﾆﾇﾈﾊﾋﾎﾏﾐﾑﾒﾓﾔﾕﾗﾘﾜ0123456789"
						class="console-text-input"
						autocomplete="off"
						spellcheck="false"
					/>
				</label>
			</div>

			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Анимация матричного дождя</h3>
					<p class="setting-row__desc">
						Если выключить — на пустом экране консоли вместо дождя будет статичный спящий волк.
					</p>
				</div>
				<Toggle v-model="modlexConsoleRainEnabled" />
			</div>

			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Цвет заполняющих символов</h3>
					<p class="setting-row__desc">
						Только для букв в матричном дожде —
						{{ modlexConsoleFillColor ? modlexConsoleFillColor : 'стандартный (серый)' }}
					</p>
				</div>
				<div class="flex items-center gap-2">
					<input
						type="color"
						class="color-picker"
						:value="modlexConsoleFillColor || DEFAULT_CONSOLE_FILL"
						@input="modlexConsoleFillColor = ($event.target as HTMLInputElement).value"
					/>
					<Button
						v-if="modlexConsoleFillColor"
						type="outlined"
						size="sm"
						native-type="button"
						@click="modlexConsoleFillColor = ''"
						>Сбросить</Button
					>
				</div>
			</div>

			<div v-if="modlexConsoleRainEnabled" class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Цвет символов дождя</h3>
					<p class="setting-row__desc">
						{{ modlexConsoleRainColor ? modlexConsoleRainColor : 'стандартный (зелёный)' }}
					</p>
				</div>
				<div class="flex items-center gap-2">
					<input
						type="color"
						class="color-picker"
						:value="modlexConsoleRainColor || DEFAULT_CONSOLE_RAIN"
						@input="modlexConsoleRainColor = ($event.target as HTMLInputElement).value"
					/>
					<Button
						v-if="modlexConsoleRainColor"
						type="outlined"
						size="sm"
						native-type="button"
						@click="modlexConsoleRainColor = ''"
						>Сбросить</Button
					>
				</div>
			</div>

			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Фон консоли</h3>
					<p class="setting-row__desc">
						Отдельно от общего фона лаунчера —
						{{ modlexConsoleBgColor ? modlexConsoleBgColor : 'стандартный цвет темы' }}
					</p>
				</div>
				<div class="flex items-center gap-2">
					<input
						type="color"
						class="color-picker"
						:value="modlexConsoleBgColor || DEFAULT_CONSOLE_BG"
						@input="modlexConsoleBgColor = ($event.target as HTMLInputElement).value"
					/>
					<Button
						v-if="modlexConsoleBgColor"
						type="outlined"
						size="sm"
						native-type="button"
						@click="modlexConsoleBgColor = ''"
						>Сбросить</Button
					>
				</div>
			</div>

			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Сбросить настройки консоли</h3>
					<p class="setting-row__desc">Вернуть надпись, символы и цвета консоли к стандартным.</p>
				</div>
				<Button type="outlined" size="sm" native-type="button" @click="resetConsoleSettings">
					Сбросить всё
				</Button>
			</div>
		</div>

		<!-- Discord -->
		<div class="settings-section">
			<h2 class="settings-section__title">Discord Rich Presence</h2>
			<p class="settings-section__desc">
				Свой текст статуса вместо "Играет {{ '{instance}' }}" — работает, если Discord Rich Presence
				включён в Настройки → Аккаунт → Приватность.
			</p>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Текст статуса</h3>
					<p class="setting-row__desc">{{ '{instance}' }} — подставится имя запущенного инстанса</p>
				</div>
				<input
					:value="discordMessage"
					type="text"
					maxlength="128"
					placeholder="Играет {instance}"
					class="console-text-input"
					autocomplete="off"
					spellcheck="false"
					@input="onDiscordMessageInput(($event.target as HTMLInputElement).value)"
				/>
			</div>

			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Текст бездействия</h3>
					<p class="setting-row__desc">Показывается, когда нет запущенного инстанса</p>
				</div>
				<input
					:value="discordIdleMessage"
					type="text"
					maxlength="128"
					placeholder="Бездействует..."
					class="console-text-input"
					autocomplete="off"
					spellcheck="false"
					@input="onDiscordIdleMessageInput(($event.target as HTMLInputElement).value)"
				/>
			</div>
		</div>

		<!-- Запуск -->
		<div v-if="multiLaunchFeatureEnabled" class="settings-section">
			<h2 class="settings-section__title">Запуск</h2>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Отключить запуск нескольких аккаунтов</h3>
					<p class="setting-row__desc">
						Убирает кнопку "Запуск как несколько аккаунтов" со страницы инстанса.
					</p>
				</div>
				<Toggle v-model="modlexHideMultiLaunch" />
			</div>
		</div>

		<!-- Обновления -->
		<div class="settings-section">
			<h2 class="settings-section__title">Обновления</h2>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Канал обновлений</h3>
					<p class="setting-row__desc">
						{{
							updateChannel === 'beta'
								? 'Сейчас вы на бета-канале — получаете тестовые версии раньше остальных.'
								: 'Публичный канал — стабильные версии.'
						}}
					</p>
				</div>
				<div
					v-if="updateChannel === 'stable'"
					class="toggle-lock-wrapper"
					:class="{ 'toggle-lock-wrapper--locked': switchToBetaLocked }"
				>
					<Button type="colored" color="brand" native-type="button" @click="onSwitchToBetaClick"
						>Включить бета-канал</Button
					>
				</div>
				<div
					v-else
					class="toggle-lock-wrapper"
					:class="{ 'toggle-lock-wrapper--locked': switchToStableLocked }"
				>
					<Button
						type="outlined"
						native-type="button"
						:disabled="switchingChannel"
						@click="onSwitchToStableClick"
					>
						Вернуться на публичный канал
					</Button>
				</div>
			</div>
		</div>

		<BetaChannelModal ref="betaModal" @approved="onBetaApproved" />

		<!-- Контент -->
		<div class="settings-section">
			<h2 class="settings-section__title">Контент</h2>
			<div class="setting-row">
				<div class="setting-row__info">
					<h3 class="setting-row__label">Источник новостей</h3>
					<p class="setting-row__desc">Откуда загружать новости в правой панели.</p>
				</div>
				<DropdownSelect
					v-model="modlexNewsSource"
					name="news-source"
					:options="newsSourceOptions"
					:get-option-label="getNewsLabel"
					class="settings-dropdown"
				/>
			</div>
		</div>

		<!-- Платформы поиска -->
		<div class="settings-section">
			<h2 class="settings-section__title">Платформы</h2>
			<p class="settings-section__desc">
				Управляйте источниками при поиске и установке модов. Хотя бы одна платформа должна
				оставаться включённой.
			</p>

			<!-- Modrinth -->
			<div class="setting-row platform-row">
				<div class="platform-logo modrinth-logo">
					<img src="https://cdn.modrinth.com/modrinth-new.png" alt="Modrinth" />
				</div>
				<div class="setting-row__info">
					<h3 class="setting-row__label">Modrinth</h3>
					<p class="setting-row__desc">Официальный магазин модов Modrinth.</p>
				</div>
				<Toggle :model-value="modlexEnableModrinth" @update:model-value="onToggleModrinth" />
			</div>

			<!-- CurseForge -->
			<div class="setting-row platform-row">
				<div class="platform-logo cf-logo">
					<svg viewBox="0 0 32 32" fill="none" xmlns="http://www.w3.org/2000/svg">
						<path d="M8 6h10l-3 7h5L9 28l3-11H7L8 6z" fill="#F16436" />
					</svg>
				</div>
				<div class="setting-row__info">
					<h3 class="setting-row__label">CurseForge</h3>
					<p class="setting-row__desc">Крупнейший архив модов для Minecraft.</p>
				</div>
				<div class="toggle-lock-wrapper" :class="{ 'toggle-lock-wrapper--locked': cfLocked }">
					<Toggle :model-value="cfDisplayValue" @update:model-value="onToggleCurseForge" />
				</div>
			</div>

			<p v-if="!modlexEnableModrinth && !cfDisplayValue" class="platform-warning">
				⚠ Включите хотя бы одну платформу.
			</p>
		</div>
	</div>
</template>

<script setup lang="ts">
import { Button, DropdownSelect, Toggle } from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import BetaChannelModal from '@/components/ui/modal/BetaChannelModal.vue'
import { useFeatureFlag } from '@/helpers/feature-flags'
import {
	exportThemeCode,
	importThemeCode,
	modlexAccentColor,
	modlexBgColor,
	modlexConsoleBgColor,
	modlexConsoleFillChar,
	modlexConsoleFillColor,
	modlexConsoleRainChars,
	modlexConsoleRainColor,
	modlexConsoleRainEnabled,
	modlexConsoleText,
	modlexDividerColor,
	modlexDoubleBorderEnabled,
	modlexDoubleBorderInner,
	modlexDoubleBorderOuter,
	modlexEnableCurseForge,
	modlexEnableModrinth,
	modlexFloatingGlassEffect,
	modlexHideFloatingAccountWidget,
	modlexHideFriends,
	modlexHideMultiLaunch,
	modlexHideMusicTab,
	modlexHideRightSidebar,
	modlexHideServers,
	modlexIconColor,
	modlexNewsSource,
	modlexPanelColor,
	modlexTextColor,
	modlexTextOutlineColor,
	modlexTextOutlineEnabled,
	type NewsSource,
	resetConsoleSettings,
} from '@/helpers/modlex-settings'
import { get as getSettings, set as setSettings } from '@/helpers/settings'
import { requestImmediateUpdateCheck } from '@/providers/app-update'

const newsSourceOptions: NewsSource[] = ['github', 'modrinth', 'off']

function getNewsLabel(value: NewsSource): string {
	return { github: 'GitHub', modrinth: 'Modrinth', off: 'Выключено' }[value] ?? value
}

const { locked: cfLocked, message: cfLockedMessage } = useFeatureFlag('curseforge_platform')
const { locked: switchToStableLocked, message: switchToStableLockedMessage } = useFeatureFlag(
	'switch_to_stable_channel',
)
const { locked: switchToBetaLocked, message: switchToBetaLockedMessage } =
	useFeatureFlag('switch_to_beta_channel')
const { enabled: musicFeatureEnabled } = useFeatureFlag('modlex_music')
const { enabled: multiLaunchFeatureEnabled } = useFeatureFlag('multi_account_launch')

// CurseForge приостановлен (сложная логика интерфейса ещё не готова), пока
// заблокирован фича-флагом — показываем тумблер выключенным, даже если
// пользователь когда-то включил его в настройках (это сохранённое значение
// вернётся, как только фича разблокируется).
const cfDisplayValue = computed(() => (cfLocked.value ? false : modlexEnableCurseForge.value))

// ===== MODLEX: акцентный цвет =====
const DEFAULT_ACCENT = '#8e32f3'
const accentColorForPicker = computed(() => modlexAccentColor.value || DEFAULT_ACCENT)
// ===== END MODLEX =====

// ===== MODLEX: расширенная кастомизация цвета =====
const showAdvancedTheme = ref(false)
const DEFAULT_BG = '#16181c'
const DEFAULT_PANEL = '#27292e'
const DEFAULT_TEXT = '#ffffff'
const DEFAULT_ICON = '#b0bac5'
const DEFAULT_CONSOLE_FILL = '#808080'
const DEFAULT_CONSOLE_RAIN = '#33ff33'
const DEFAULT_CONSOLE_BG = '#16181c'
const DEFAULT_DIVIDER = '#34363c'
// ===== END MODLEX =====

// ===== MODLEX: превью консоли запуска — временно отключено, см. шаблон выше =====
// const previewTerminal = ref<InstanceType<typeof BaseTerminal>>()
// ===== END MODLEX =====

const inlineNotice = ref<string | null>(null)
let inlineNoticeTimeout: ReturnType<typeof setTimeout> | null = null

function showInlineNotice(text: string) {
	if (inlineNoticeTimeout) clearTimeout(inlineNoticeTimeout)
	inlineNotice.value = text
	inlineNoticeTimeout = setTimeout(() => {
		inlineNotice.value = null
	}, 6000)
}

// ===== MODLEX: экспорт/импорт кода темы =====
const importThemeCodeInput = ref('')

async function copyThemeCode() {
	const code = exportThemeCode()
	try {
		await navigator.clipboard.writeText(code)
		showInlineNotice('Код темы скопирован в буфер обмена')
	} catch {
		console.log('[ModLEX] код темы:', code)
		showInlineNotice('Не удалось скопировать — код выведен в консоль')
	}
}

function applyThemeCodeInput() {
	if (!importThemeCodeInput.value.trim()) return
	const ok = importThemeCode(importThemeCodeInput.value)
	showInlineNotice(ok ? 'Тема применена' : 'Не удалось прочитать код темы')
	if (ok) importThemeCodeInput.value = ''
}
// ===== END MODLEX =====

// ===== MODLEX: канал обновлений =====
const updateChannel = ref<'stable' | 'beta'>('stable')
const switchingChannel = ref(false)
const betaModal = ref<InstanceType<typeof BetaChannelModal>>()

onMounted(async () => {
	updateChannel.value = (await getSettings()).modlex_update_channel ?? 'stable'
})

async function onBetaApproved() {
	switchingChannel.value = true
	try {
		const settings = await getSettings()
		settings.modlex_update_channel = 'beta'
		settings.modlex_beta_verified = true
		await setSettings(settings)
		updateChannel.value = 'beta'
		// Небольшая задержка — чтобы пользователь успел увидеть подтверждение
		// в модалке перед тем, как она закроется.
		setTimeout(() => betaModal.value?.hide(), 1200)
		// Немедленно перепроверяет обновления на новом канале и, если что-то
		// найдётся, сама скачивает и ставит — без ручного подтверждения
		// "Перезапустить и обновить", в отличие от обычного публичного канала.
		await requestImmediateUpdateCheck()
	} finally {
		switchingChannel.value = false
	}
}

async function returnToStableChannel() {
	switchingChannel.value = true
	try {
		const settings = await getSettings()
		settings.modlex_update_channel = 'stable'
		await setSettings(settings)
		updateChannel.value = 'stable'
	} finally {
		switchingChannel.value = false
	}
}

function onSwitchToBetaClick() {
	if (switchToBetaLocked.value) {
		showInlineNotice(switchToBetaLockedMessage.value)
		return
	}
	betaModal?.value?.show()
}

function onSwitchToStableClick() {
	if (switchToStableLocked.value) {
		showInlineNotice(switchToStableLockedMessage.value)
		return
	}
	returnToStableChannel()
}
// ===== END MODLEX =====

// ===== MODLEX: текст Discord Rich Presence =====
const discordMessage = ref('')
const discordIdleMessage = ref('')
let discordMessageSaveTimeout: ReturnType<typeof setTimeout> | null = null
let discordIdleMessageSaveTimeout: ReturnType<typeof setTimeout> | null = null

onMounted(async () => {
	const settings = await getSettings()
	discordMessage.value = settings.modlex_discord_message ?? ''
	discordIdleMessage.value = settings.modlex_discord_idle_message ?? ''
})

function onDiscordMessageInput(value: string) {
	discordMessage.value = value
	if (discordMessageSaveTimeout) clearTimeout(discordMessageSaveTimeout)
	discordMessageSaveTimeout = setTimeout(async () => {
		const settings = await getSettings()
		settings.modlex_discord_message = value.trim() || null
		await setSettings(settings)
	}, 500)
}

function onDiscordIdleMessageInput(value: string) {
	discordIdleMessage.value = value
	if (discordIdleMessageSaveTimeout) clearTimeout(discordIdleMessageSaveTimeout)
	discordIdleMessageSaveTimeout = setTimeout(async () => {
		const settings = await getSettings()
		settings.modlex_discord_idle_message = value.trim() || null
		await setSettings(settings)
	}, 500)
}

// ===== END MODLEX =====

onBeforeUnmount(() => {
	if (inlineNoticeTimeout) clearTimeout(inlineNoticeTimeout)
	if (discordMessageSaveTimeout) clearTimeout(discordMessageSaveTimeout)
	if (discordIdleMessageSaveTimeout) clearTimeout(discordIdleMessageSaveTimeout)
})

function onToggleModrinth(value: boolean) {
	if (!value && modlexEnableModrinth.value && !cfDisplayValue.value) {
		showInlineNotice(
			'Нельзя отключить: должна остаться включённой хотя бы одна платформа, а CurseForge сейчас недоступен.',
		)
		return
	}
	modlexEnableModrinth.value = value
}

function onToggleCurseForge(value: boolean) {
	if (cfLocked.value) {
		showInlineNotice(cfLockedMessage.value)
		return
	}
	if (!value && modlexEnableCurseForge.value && !modlexEnableModrinth.value) {
		showInlineNotice('Нельзя отключить: должна остаться включённой хотя бы одна платформа.')
		return
	}
	modlexEnableCurseForge.value = value
}
</script>

<style scoped>
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

.platform-row {
	gap: 0.75rem;
}

.platform-logo {
	width: 2rem;
	height: 2rem;
	flex-shrink: 0;
	border-radius: 0.375rem;
	display: flex;
	align-items: center;
	justify-content: center;
	overflow: hidden;
}

.platform-logo img,
.platform-logo svg {
	width: 100%;
	height: 100%;
	object-fit: contain;
}

.modrinth-logo {
	background: #1bd96a1a;
}

.cf-logo {
	background: #f164361a;
}

.settings-dropdown {
	flex-shrink: 0;
	min-width: 140px;
}

.platform-warning {
	margin: 0.5rem 0 0;
	font-size: 0.8rem;
	color: var(--color-orange);
}

.toggle-lock-wrapper--locked {
	opacity: 0.5;
	filter: grayscale(1);
}

.toggle-lock-wrapper--locked :deep(button) {
	cursor: not-allowed;
}

.settings-notice {
	width: fit-content;
	max-width: 26rem;
	margin-bottom: 0.5rem;
	padding: 0.6rem 1.1rem;
	border-radius: 999px;
	border: 1px solid rgba(255, 255, 255, 0.25);
	background: rgba(59, 130, 246, 0.28);
	backdrop-filter: blur(16px);
	-webkit-backdrop-filter: blur(16px);
	box-shadow: 0 4px 20px rgba(0, 0, 0, 0.25);
	color: #fff;
	font-size: 0.875rem;
	font-weight: 600;
	text-align: center;
}

.settings-notice-fade-enter-active,
.settings-notice-fade-leave-active {
	transition:
		opacity 0.2s ease,
		transform 0.2s ease;
}

.settings-notice-fade-enter-from,
.settings-notice-fade-leave-to {
	opacity: 0;
	transform: translateY(-0.5rem);
}

.advanced-theme {
	margin-top: 0.75rem;
	padding-top: 0.75rem;
	border-top: 1px solid var(--color-divider);
}

.color-picker {
	width: 2.25rem;
	height: 2.25rem;
	padding: 0;
	border: 1px solid var(--color-divider);
	border-radius: 0.5rem;
	background: none;
	cursor: pointer;
}

.color-picker::-webkit-color-swatch-wrapper {
	padding: 2px;
}

.color-picker::-webkit-color-swatch {
	border: none;
	border-radius: 0.375rem;
}

.console-settings-controls {
	display: flex;
	flex-direction: column;
	gap: 1rem;
	margin-top: 1rem;
}

.console-field {
	display: flex;
	flex-direction: column;
	gap: 0.375rem;
}

.console-field__label {
	font-size: 0.875rem;
	font-weight: 600;
	color: var(--color-contrast);
}

.console-text-input {
	padding: 0.5rem 0.75rem;
	border: 1px solid var(--color-divider);
	border-radius: 0.5rem;
	background: var(--color-button-bg);
	color: var(--color-contrast);
	font-size: 0.875rem;
}

.console-text-input--narrow {
	width: 3rem;
	text-align: center;
}

.console-preview {
	width: 100%;
	height: 260px;
	border-radius: 0.75rem;
	overflow: hidden;
}
</style>
