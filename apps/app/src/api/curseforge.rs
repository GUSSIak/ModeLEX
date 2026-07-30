//! Tauri-команды для CurseForge
//! apps/app/src/api/curseforge.rs

use tauri::plugin::TauriPlugin;
use tauri::Runtime;
use theseus::api::curseforge::{self, CfMod, CfFile, CfSearchResult, CfCategory, CfModDetails};
use crate::api::Result;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("curseforge")
        .invoke_handler(tauri::generate_handler![
            cf_search,
            cf_get_mod,
            cf_get_mod_details,
            cf_get_files,
            cf_get_description,
            cf_install_mod,
            cf_install_modpack,
            cf_remove_mod,
            cf_get_categories,
            cf_clear_cache,
        ])
        .build()
}

#[tauri::command]
pub async fn cf_search(
    query: String,
    game_version: Option<String>,
    loader: Option<String>,
    class_id: Option<u32>,
    sort_field: Option<String>,
    sort_order: Option<String>,
    page: Option<u32>,
    category_id: Option<u32>,
) -> Result<CfSearchResult> {
    Ok(curseforge::search(
        &query,
        game_version.as_deref(),
        loader.as_deref(),
        class_id,
        sort_field.as_deref(),
        sort_order.as_deref(),
        page.unwrap_or(0),
        category_id,
    )
    .await?)
}

#[tauri::command]
pub async fn cf_get_mod(mod_id: u32) -> Result<CfMod> {
    Ok(curseforge::get_mod(mod_id).await?)
}

#[tauri::command]
pub async fn cf_get_mod_details(mod_id: u32) -> Result<CfModDetails> {
    Ok(curseforge::get_mod_details(mod_id).await?)
}

#[tauri::command]
pub async fn cf_get_files(
    mod_id: u32,
    game_version: Option<String>,
    loader: Option<String>,
) -> Result<Vec<CfFile>> {
    Ok(curseforge::get_files(mod_id, game_version.as_deref(), loader.as_deref()).await?)
}

#[tauri::command]
pub async fn cf_install_mod(
    profile_path: String,
    mod_id: u32,
    file_id: Option<u32>,
    game_version: String,
    loader: String,
) -> Result<Vec<u32>> {
    Ok(curseforge::install_mod(
        &profile_path,
        mod_id,
        file_id,
        &game_version,
        &loader,
    )
    .await?)
}

#[tauri::command]
pub async fn cf_remove_mod(profile_path: String, mod_id: u32) -> Result<()> {
    use theseus::api::curseforge::read_sidecar;
    use theseus::api::instance::get_full_path;

    let mut sidecar = read_sidecar(&profile_path).await;
    let to_remove: Vec<String> = sidecar
        .0
        .iter()
        .filter(|(_, m)| m.mod_id == mod_id)
        .map(|(hash, _)| hash.clone())
        .collect();

    let profile_base = get_full_path(&profile_path).await?;
    for hash in &to_remove {
        if let Some(meta) = sidecar.0.get(hash) {
            let file_path = profile_base.join("mods").join(&meta.file_name);
            let _ = tokio::fs::remove_file(&file_path).await;
            let disabled = profile_base
                .join("mods")
                .join(format!("{}.disabled", meta.file_name));
            let _ = tokio::fs::remove_file(&disabled).await;
        }
    }

    for hash in &to_remove {
        sidecar.0.remove(hash);
    }

    let sidecar_path = profile_base.join(".modlex").join("cf_mods.json");
    let json = serde_json::to_string_pretty(&sidecar).map_err(|e| {
        theseus::ErrorKind::OtherError(e.to_string()).as_error()
    })?;
    tokio::fs::write(&sidecar_path, json).await.map_err(|e| {
        theseus::ErrorKind::OtherError(e.to_string()).as_error()
    })?;

    Ok(())
}

#[tauri::command]
pub async fn cf_get_description(mod_id: u32) -> Result<String> {
    Ok(curseforge::get_mod_description(mod_id).await?)
}

#[tauri::command]
pub async fn cf_install_modpack(
    mod_id: u32,
    file_id: Option<u32>,
    game_version: String,
    loader: String,
) -> Result<String> {
    Ok(curseforge::install_modpack(mod_id, file_id, &game_version, &loader).await?)
}

#[tauri::command]
pub async fn cf_get_categories(class_id: Option<u32>) -> Result<Vec<CfCategory>> {
    Ok(curseforge::get_categories(class_id).await?)
}

#[tauri::command]
pub async fn cf_clear_cache() -> Result<()> {
    Ok(curseforge::clear_cache().await?)
}
