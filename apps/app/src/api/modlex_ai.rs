//! Tauri-команды для ИИ-агента ModLEX
//! apps/app/src/api/modlex_ai.rs

use crate::api::Result;
use tauri::Emitter;
use theseus::modlex_ai::{
    AgentStepOutcome, AiChatMessage, AiModelInfo, AiToolCall, KeyPingResult,
};
use theseus::prelude::*;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    // Идентификатор плагина ОБЯЗАН быть kebab-case — Tauri ACL не допускает
    // подчёркивания вообще нигде в identifier (даже до двоеточия), только
    // строчные буквы и дефисы. Отсюда "modlex-ai", а не "modlex_ai" (имя
    // файла/модуля может оставаться со снейк-кейсом, это просто Rust-модуль).
    tauri::plugin::Builder::<R>::new("modlex-ai")
        .invoke_handler(tauri::generate_handler![
            modlex_ai_agent_step,
            modlex_ai_agent_resume,
            modlex_ai_list_models,
            modlex_ai_send_bug_report,
            modlex_ai_dev_ping_available,
            modlex_ai_ping_key_pool,
            modlex_ai_ping_own_key,
        ])
        .build()
}

/// Пока идёт длинный ход (особенно с auto_confirm — там нет пауз на
/// подтверждение между шагами), фронт иначе не видит НИЧЕГО до самого конца
/// всего хода целиком, и кажется, что запрос завис (и весь текст с пузырями
/// инструментов "вываливается" одним пакетом). Пробрасываем КАЖДОЕ сообщение
/// (ответ ассистента, результат каждого инструмента) как отдельное
/// Tauri-событие сразу же, не дожидаясь ответа всей команды — простой
/// emit/listen в обход сложной кастомной AppEvent/postcard-шины, которой
/// пользуется остальной лаунчер (она не рассчитана на что-то настолько
/// специфичное и одноразовое).
fn spawn_hop_forwarder<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
) -> tokio::sync::mpsc::UnboundedSender<AiChatMessage> {
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AiChatMessage>();
    tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            let _ = app.emit("modlex-ai-hop", message);
        }
    });
    tx
}

#[tauri::command]
pub async fn modlex_ai_agent_step<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    messages: Vec<AiChatMessage>,
) -> Result<AgentStepOutcome> {
    let settings = settings::get().await?;
    let outcome = theseus::modlex_ai::agent_step(
        messages,
        settings.modlex_ai_api_key,
        &settings.modlex_ai_model,
        &settings.modlex_ai_provider,
        settings.modlex_ai_auto_confirm,
        settings.modlex_ai_max_hops,
        Some(spawn_hop_forwarder(app)),
    )
    .await?;
    Ok(outcome)
}

/// Продолжает диалог после подтверждения/отклонения пользователем действия
/// из `AgentStepOutcome::PendingConfirmation` (Фаза 2 — тир Confirm).
#[tauri::command]
pub async fn modlex_ai_agent_resume<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    messages: Vec<AiChatMessage>,
    pending_call: AiToolCall,
    approved: bool,
) -> Result<AgentStepOutcome> {
    let settings = settings::get().await?;
    let outcome = theseus::modlex_ai::agent_resume(
        messages,
        pending_call,
        approved,
        settings.modlex_ai_api_key,
        &settings.modlex_ai_model,
        &settings.modlex_ai_provider,
        settings.modlex_ai_auto_confirm,
        settings.modlex_ai_max_hops,
        Some(spawn_hop_forwarder(app)),
    )
    .await?;
    Ok(outcome)
}

#[tauri::command]
pub async fn modlex_ai_list_models() -> Result<Vec<AiModelInfo>> {
    let models = theseus::modlex_ai::list_models().await?;
    Ok(models)
}

#[tauri::command]
pub async fn modlex_ai_send_bug_report(
    title: String,
    description: String,
    fields: Vec<(String, String)>,
) -> Result<()> {
    theseus::modlex_ai::send_bug_report(&title, &description, fields).await?;
    Ok(())
}

/// Tells the frontend whether to even show the "проверить весь пул ключей"
/// button — a compile-time flag, see modlex_ai.rs's module doc for why this
/// isn't a devMode/Settings check.
#[tauri::command]
pub fn modlex_ai_dev_ping_available() -> bool {
    theseus::modlex_ai::dev_key_ping_available()
}

/// Developer-only diagnostic: pings every built-in shared key across all
/// three providers. Refuses at the theseus layer too if the compile-time
/// flag isn't set, regardless of what the frontend sends.
#[tauri::command]
pub async fn modlex_ai_ping_key_pool() -> Result<Vec<KeyPingResult>> {
    Ok(theseus::modlex_ai::ping_key_pool().await?)
}

/// Pings only the caller's own z.ai key (from Settings) — safe for any user,
/// spends only their own quota.
#[tauri::command]
pub async fn modlex_ai_ping_own_key(key: String) -> Result<KeyPingResult> {
    Ok(theseus::modlex_ai::ping_own_key(key).await?)
}
