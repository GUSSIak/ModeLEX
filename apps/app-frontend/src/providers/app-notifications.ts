import {
	AbstractWebNotificationManager,
	type NotificationPanelLocation,
	type WebNotification,
} from '@modrinth/ui'
import { type Ref, ref } from 'vue'

import { requestAiAgentHelp } from '@/composables/use-ai-agent-bridge'

export class AppNotificationManager extends AbstractWebNotificationManager {
	// MODLEX: переопределяем базовый handleError() — тот же текст/заголовок,
	// но с добавленной кнопкой "Спросить ИИ", которая открывает ИИ-агента и
	// сразу отправляет ему текст ошибки, без изменений в packages/ui (общем
	// с веб-версией Modrinth).
	handleError = (error: Error): void => {
		const message = (error as { message?: string })?.message ?? String(error)
		this.addNotification({
			title: 'An error occurred',
			text: message,
			type: 'error',
			buttons: [
				{
					label: 'Спросить ИИ',
					keepOpen: true,
					action: () =>
						requestAiAgentHelp(`У меня в лаунчере ошибка: "${message}". Нужна помощь с этим.`),
				},
			],
		})
	}

	private readonly state: Ref<WebNotification[]>
	private readonly locationState: Ref<NotificationPanelLocation>

	public constructor() {
		super()
		this.state = ref<WebNotification[]>([])
		this.locationState = ref<NotificationPanelLocation>('right')
	}

	public getNotificationLocation(): NotificationPanelLocation {
		return this.locationState.value
	}

	public setNotificationLocation(location: NotificationPanelLocation): void {
		this.locationState.value = location
	}

	public getNotifications(): WebNotification[] {
		return this.state.value
	}

	protected addNotificationToStorage(notification: WebNotification): void {
		this.state.value.push(notification)
	}

	protected removeNotificationFromStorage(id: string | number): void {
		const index = this.state.value.findIndex((n) => n.id === id)
		if (index > -1) {
			this.state.value.splice(index, 1)
		}
	}

	protected removeNotificationFromStorageByIndex(index: number): void {
		this.state.value.splice(index, 1)
	}

	protected clearAllNotificationsFromStorage(): void {
		this.state.value.splice(0)
	}
}
