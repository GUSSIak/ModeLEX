// ===== MODLEX: PostHog (Modrinth-телеметрия) полностью отключена =====
// Раньше здесь был import { posthog } from 'posthog-js' и реальные вызовы
// posthog.init/.capture/.opt_in_capturing/.opt_out_capturing/.debug(). Ниже —
// те же экспорты (сигнатуры сохранены, чтобы не трогать ~70 мест вызова
// trackEvent(...) по всему приложению и переключатель в PrivacySettings.vue),
// но все они теперь no-op — никуда ничего не отправляется.

interface InstanceProperties {
	loader: string
	game_version: string
}

interface ProjectProperties extends InstanceProperties {
	id: string
	project_type: string
}

type AnalyticsEventMap = {
	Launched: { version: string; dev: boolean }
	PageView: { path: string; fromPath: string; failed: unknown }
	InstanceCreate: { source: string }
	InstanceCreateStart: { source: string }
	InstanceStart: InstanceProperties & { source: string }
	InstanceStop: Partial<InstanceProperties> & { source?: string }
	InstanceDuplicate: InstanceProperties
	InstanceRepair: InstanceProperties
	InstanceSetIcon: Record<string, never>
	InstanceRemoveIcon: Record<string, never>
	InstanceUpdateAll: InstanceProperties & { count: number; selected: boolean }
	InstanceProjectUpdate: InstanceProperties & { id: string; name: string; project_type: string }
	InstanceProjectDisable: InstanceProperties & {
		id: string
		name: string
		project_type: string
		disabled: boolean
	}
	InstanceProjectRemove: InstanceProperties & { id: string; name: string; project_type: string }
	ProjectInstall: ProjectProperties & { version_id: string; title: string; source: string }
	ProjectInstallStart: { source: string }
	PackInstall: { id: string; version_id: string; title: string; source: string }
	PackInstallStart: Record<string, never>
	AccountLogIn: { source?: string }
	AccountLogOut: Record<string, never>
	JavaTest: { path: string; success: boolean }
	JavaManualSelect: { version: string }
	JavaAutoDetect: { path: string; version: string }
}

export type AnalyticsEvent = keyof AnalyticsEventMap

export const initAnalytics = () => {}

export const debugAnalytics = () => {}

export const optOutAnalytics = () => {}

export const optInAnalytics = () => {}

type OptionalArgs<T> = Record<string, never> extends T ? [properties?: T] : [properties: T]

export const trackEvent = <E extends AnalyticsEvent>(
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	eventName: E,
	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	...args: OptionalArgs<AnalyticsEventMap[E]>
) => {}
