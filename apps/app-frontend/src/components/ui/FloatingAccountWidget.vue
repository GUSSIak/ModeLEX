<!-- ModLEX: показывается вместо правой панели, когда она скрыта в настройках
     (Настройки → ModLEX → Внешний вид). Просто оборачивает AccountsCard в
     плавающий позиционированный контейнер — раскрывающийся вид аккаунта
     (аватар/имя/тип + шеврон) у неё уже готов, ничего своего не рисуем.
     Если modlexHideFloatingAccountWidget включён — плашка уезжает за правый
     край экрана, оставляя небольшой видимый "хвостик", и выезжает обратно
     при наведении (полностью пропадать не должна — иначе непонятно, что она
     вообще есть). -->
<script setup lang="ts">
import AccountsCard from '@/components/ui/AccountsCard.vue'
import {
	modlexFloatingGlassEffect,
	modlexHideFloatingAccountWidget,
} from '@/helpers/modlex-settings'
</script>

<template>
	<div
		class="floating-account-widget"
		:class="{
			'floating-account-widget--auto-hide': modlexHideFloatingAccountWidget,
			'floating-account-widget--glass': modlexFloatingGlassEffect,
		}"
	>
		<suspense>
			<AccountsCard />
		</suspense>
	</div>
</template>

<style scoped>
.floating-account-widget {
	position: fixed;
	top: calc(var(--top-bar-height, 3rem) + 0.75rem);
	right: 1rem;
	z-index: 40;
	width: 260px;
	transition: transform 0.25s ease-out;
}

.floating-account-widget--auto-hide {
	transform: translateX(calc(100% - 0.875rem));
}

.floating-account-widget--auto-hide:hover {
	transform: translateX(0);
}

/* ModLEX: "эффект стекла" — AccountsCard красит себя сплошным bg-button-bg
   изнутри (Tailwind-класс), поэтому переопределяем его снаружи через :deep. */
.floating-account-widget--glass :deep(.bg-button-bg) {
	background: color-mix(in srgb, var(--color-button-bg) 55%, transparent);
	backdrop-filter: blur(16px) saturate(160%);
	-webkit-backdrop-filter: blur(16px) saturate(160%);
}
</style>
