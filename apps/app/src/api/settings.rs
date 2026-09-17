use crate::api::Result;
use tauri::Runtime;
use theseus::modlex_music::LocalMusicFile;
use theseus::prelude::*;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("settings")
        .invoke_handler(tauri::generate_handler![
            settings_get,
            settings_set,
            cancel_directory_change,
            modlex_list_local_music_files,
            modlex_can_write_hosts_file,
            modlex_restore_hosts_file
        ])
        .build()
}

// Get full settings
// invoke('plugin:settings|settings_get')
#[tauri::command]
pub async fn settings_get() -> Result<Settings> {
    let res = settings::get().await?;
    Ok(res)
}

// Set full settings
// invoke('plugin:settings|settings_set', settings)
#[tauri::command]
pub async fn settings_set(settings: Settings) -> Result<()> {
    settings::set(settings).await?;
    Ok(())
}

#[tauri::command]
pub async fn cancel_directory_change<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<()> {
    let identifier = &app.config().identifier;
    settings::cancel_directory_change(identifier).await?;
    Ok(())
}

#[tauri::command]
pub async fn modlex_list_local_music_files(
    folder: &str,
) -> Result<Vec<LocalMusicFile>> {
    let res = theseus::modlex_music::list_local_music_files(folder).await?;
    Ok(res)
}

#[tauri::command]
pub async fn modlex_can_write_hosts_file() -> bool {
    theseus::settings::modlex_can_write_hosts_file().await
}

#[tauri::command]
pub async fn modlex_restore_hosts_file() -> Result<bool> {
    let res = theseus::settings::modlex_restore_hosts_file().await?;
    Ok(res)
}
