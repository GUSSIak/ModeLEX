// apps/app-frontend/src/helpers/modlex-ping.ts
//
// ВРЕМЕННО ОТКЛЮЧЕНО: раньше отправлял { id } (случайный UUID, не привязанный
// к аккаунту) на ping.modlex.deno.net раз при старте и затем каждые 90с, пока
// приложение открыто — чтобы прикидывать число активных установок. Сигнатуры
// startPing/stopPing сохранены (вызываются из App.vue), но теперь no-op.

export function startPing(): void {}

export function stopPing(): void {}
