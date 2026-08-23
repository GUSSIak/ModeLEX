use crate::api::Result;
use crate::api::oauth_utils;
use chrono::{Duration, Utc};
use tauri::plugin::TauriPlugin;
use tauri::{Manager, Runtime, UserAttentionType};
use tauri_plugin_opener::OpenerExt;
use theseus::prelude::*;
use tokio::sync::oneshot;

#[tauri::command]
pub async fn check_reachable() -> Result<()> {
    minecraft_auth::check_reachable().await?;
    Ok(())
}

#[tauri::command]
pub async fn offline_login(name: &str) -> Result<Credentials> {
    let credentials = minecraft_auth::offline_login(name).await?;
    Ok(credentials)
}

#[tauri::command]
pub async fn login<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<Credentials>> {
    let flow = minecraft_auth::begin_login().await?;

    let start = Utc::now();

    if let Some(window) = app.get_webview_window("signin") {
        let _ = window.close();
    }

    let window = tauri::WebviewWindowBuilder::new(
        &app,
        "signin",
        tauri::WebviewUrl::External(flow.auth_request_uri.parse().map_err(
            |_| {
                theseus::ErrorKind::OtherError(
                    "Error parsing auth redirect URL".to_string(),
                )
                .as_error()
            },
        )?),
    )
    .title("Sign into Modrinth")
    .always_on_top(true)
    .min_inner_size(500.0, 500.0)
    .inner_size(1000.0, 700.0)
    .focused(true)
    .center()
    .build()?;

    let _ = window.request_user_attention(Some(UserAttentionType::Critical));

    while (Utc::now() - start) < Duration::minutes(10) {
        if window.title().is_err() {
            return Ok(None);
        }

        if let Ok(url) = window.url() {
            if url
                .as_str()
                .starts_with("https://login.live.com/oauth20_desktop.srf")
                && let Some((_, code)) =
                    url.query_pairs().find(|x| x.0 == "code")
            {
                let _ = window.close();
                let val = minecraft_auth::finish_login(&code.clone(), flow).await?;
                return Ok(Some(val));
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let _ = window.close();
    Ok(None)
}

#[tauri::command]
pub async fn elyby_login<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Option<Credentials>> {
    let flow = theseus::elyby_auth::begin_login()?;

    let (auth_code_recv_socket_tx, auth_code_recv_socket) = oneshot::channel();
    let auth_code = tokio::spawn(oauth_utils::auth_code_reply::listen(
        auth_code_recv_socket_tx,
        Some(theseus::elyby_auth::REDIRECT_PORT),
    ));

    auth_code_recv_socket.await.unwrap()?;

    app.opener()
        .open_url(flow.auth_request_uri, None::<&str>)
        .map_err(|e| {
            theseus::ErrorKind::OtherError(format!(
                "Error opening Ely.by sign-in page: {e}"
            ))
            .as_error()
        })?;

    let Some(code) = auth_code.await.unwrap()? else {
        return Ok(None);
    };

    let credentials = theseus::elyby_auth::finish_login(&code).await?;

    if let Some(window) = app.get_window("main") {
        window.set_focus().ok();
    }

    Ok(Some(credentials))
}

#[tauri::command]
pub fn cancel_elyby_login() {
    oauth_utils::auth_code_reply::stop_listeners();
}

#[tauri::command]
pub async fn remove_user(user: uuid::Uuid) -> Result<()> {
    Ok(minecraft_auth::remove_user(user).await?)
}

#[tauri::command]
pub async fn get_default_user() -> Result<Option<uuid::Uuid>> {
    Ok(minecraft_auth::get_default_user().await?)
}

#[tauri::command]
pub async fn set_default_user(user: uuid::Uuid) -> Result<()> {
    Ok(minecraft_auth::set_default_user(user).await?)
}

#[tauri::command]
pub async fn get_users() -> Result<Vec<Credentials>> {
    Ok(minecraft_auth::users().await?)
}

/// ModLEX: Fetches the current skin texture URL for a specific account, regardless
/// of account kind or whether it's the active/default account. `Credentials.offline_profile`
/// is only ever populated at login time (empty skins for Ely.by/offline), so the account
/// list needs this to render real Ely.by avatars instead of always falling back to Steve.
#[tauri::command]
pub async fn get_account_skin_texture_url(
    user: uuid::Uuid,
) -> Result<Option<String>> {
    let users = minecraft_auth::users().await?;
    let Some(account) = users.into_iter().find(|c| c.offline_profile.id == user)
    else {
        return Ok(None);
    };

    let Some(profile) = account.online_profile().await else {
        return Ok(None);
    };

    Ok(profile.skins.first().map(|skin| skin.url.to_string()))
}

/// ModLEX: Forces a fresh online-profile fetch for a specific account, bypassing
/// the profile cache entirely. Used by the "Update skin" button for Ely.by
/// accounts — those are edited on ely.by's own website, so the app has no other
/// signal that the skin changed and would otherwise keep serving cached data.
#[tauri::command]
pub async fn refresh_account_skin(user: uuid::Uuid) -> Result<()> {
    let users = minecraft_auth::users().await?;
    let Some(account) = users.into_iter().find(|c| c.offline_profile.id == user)
    else {
        return Ok(());
    };

    let _ = account.refresh_online_profile().await;
    Ok(())
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("auth")
        .invoke_handler(tauri::generate_handler![
            check_reachable,
            login,
            offline_login,
            elyby_login,
            cancel_elyby_login,
            remove_user,
            get_default_user,
            set_default_user,
            get_users,
            get_account_skin_texture_url,
            refresh_account_skin,
        ])
        .build()
}
