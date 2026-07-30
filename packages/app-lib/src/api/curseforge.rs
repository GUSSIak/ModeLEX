//! packages/app-lib/src/api/curseforge.rs
//! CurseForge API client + install logic

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::State;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::util::fetch::sha1_async;

const CF_BASE: &str = "https://api.curseforge.com";
const MINECRAFT_GAME_ID: u32 = 432;
const CACHE_TTL: Duration = Duration::from_secs(600); // 10 минут

#[derive(Clone)]
struct CacheEntry<T> {
    data: T,
    timestamp: Instant,
}

#[derive(Clone)]
pub struct CfCache {
    categories: Arc<Mutex<Option<CacheEntry<Vec<CfCategory>>>>>,
    mods: Arc<Mutex<HashMap<u32, CacheEntry<CfMod>>>>,
    details: Arc<Mutex<HashMap<u32, CacheEntry<CfModDetails>>>>,
    search: Arc<Mutex<HashMap<String, CacheEntry<CfSearchResult>>>>,
}

impl CfCache {
    pub fn new() -> Self {
        Self {
            categories: Arc::new(Mutex::new(None)),
            mods: Arc::new(Mutex::new(HashMap::new())),
            details: Arc::new(Mutex::new(HashMap::new())),
            search: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn is_valid<T>(entry: &CacheEntry<T>) -> bool {
        entry.timestamp.elapsed() < CACHE_TTL
    }

    pub fn clear(&self) {
        self.categories.lock().unwrap().take();
        self.mods.lock().unwrap().clear();
        self.details.lock().unwrap().clear();
        self.search.lock().unwrap().clear();
        tracing::info!("CurseForge cache cleared");
    }
}

// Глобальный кеш
lazy_static::lazy_static! {
    static ref CACHE: CfCache = CfCache::new();
}

#[instrument]
pub async fn clear_cache() -> crate::Result<()> {
    CACHE.clear();
    Ok(())
}

fn get_api_key() -> &'static str {
    static KEY: OnceLock<String> = OnceLock::new();
    KEY.get_or_init(|| {
        let env_path = std::path::Path::new("D:\\ModLEX\\env\\.env");
        if !env_path.exists() {
            tracing::error!("❌ .env file NOT found at: {:?}", env_path);
            return String::new();
        }
        match std::fs::read_to_string(env_path) {
            Ok(content) => {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("CURSEFORGE_API_KEY=") {
                        let key = line.trim_start_matches("CURSEFORGE_API_KEY=").trim();
                        if !key.is_empty() {
                            return key.to_string();
                        }
                    }
                }
                tracing::error!("❌ CURSEFORGE_API_KEY not found in .env file!");
                String::new()
            }
            Err(e) => {
                tracing::error!("❌ Failed to read .env file: {}", e);
                String::new()
            }
        }
    })
}

fn cf_headers() -> HeaderMap {
    let mut h = HeaderMap::new();
    let key = get_api_key();
    if key.is_empty() {
        tracing::error!("❌ CurseForge API key is empty!");
        return h;
    }
    let _ = HeaderValue::from_str(key).map(|val| {
        h.insert("x-api-key", val);
    });
    h.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    h.insert("Accept", HeaderValue::from_static("application/json"));
    h.insert("User-Agent", HeaderValue::from_static("ModLEX/1.0 (https://github.com/GUSSIak/ModeLEX)"));
    h
}

fn cf_client() -> reqwest::Client {
    reqwest::Client::builder()
        .default_headers(cf_headers())
        .timeout(Duration::from_secs(30))
        .build()
        .expect("CF reqwest client")
}

// ── Типы данных ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfMod {
    pub id: u32,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub logo: Option<CfLogo>,
    pub download_count: f64,
    pub date_modified: String,
    pub categories: Vec<CfCategory>,
    pub authors: Vec<CfAuthor>,
    pub links: Option<CfLinks>,
    pub class_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfLogo {
    pub url: String,
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfCategory {
    pub id: u32,
    pub name: String,
    pub class_id: Option<u32>,
    pub parent_category_id: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfAuthor {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfLinks {
    pub website_url: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfFile {
    pub id: u32,
    pub mod_id: u32,
    pub display_name: String,
    pub file_name: String,
    pub download_url: Option<String>,
    pub file_length: u64,
    pub game_versions: Vec<String>,
    pub dependencies: Vec<CfDependency>,
    pub file_date: String,
    pub hashes: Vec<CfFileHash>,
    pub release_type: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CfFileHash {
    pub value: String,
    pub algo: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfDependency {
    pub mod_id: u32,
    pub relation_type: u8,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfSearchResult {
    pub data: Vec<CfMod>,
    pub pagination: CfPagination,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfPagination {
    pub index: u32,
    pub page_size: u32,
    pub result_count: u32,
    pub total_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfModDetails {
    #[serde(rename = "mod")]
    pub mod_data: CfMod,
    pub description: String,
    pub screenshots: Vec<CfScreenshot>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfScreenshot {
    pub id: u32,
    pub url: String,
    pub thumbnail_url: String,
    pub title: String,
}

// ── Сайдкар ─────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CfSidecar(pub HashMap<String, CfInstalledMeta>);

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CfInstalledMeta {
    pub mod_id: u32,
    pub file_id: u32,
    pub title: String,
    pub icon_url: Option<String>,
    pub file_name: String,
    pub version: String,
    pub author: String,
    pub game_version: String,
}

async fn sidecar_path(profile_path: &str) -> crate::Result<PathBuf> {
    let base = crate::api::instance::get_full_path(profile_path).await?;
    Ok(base.join(".modlex").join("cf_mods.json"))
}

pub async fn read_sidecar(profile_path: &str) -> CfSidecar {
    let Ok(path) = sidecar_path(profile_path).await else {
        return CfSidecar::default();
    };
    let Ok(raw) = tokio::fs::read_to_string(&path).await else {
        return CfSidecar::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

async fn write_sidecar(profile_path: &str, sidecar: &CfSidecar) -> crate::Result<()> {
    let path = sidecar_path(profile_path).await?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            crate::ErrorKind::OtherError(format!("Failed to create .modlex dir: {e}")).as_error()
        })?;
    }
    let json = serde_json::to_string_pretty(sidecar)?;
    tokio::fs::write(&path, json).await.map_err(|e| {
        crate::ErrorKind::OtherError(format!("Failed to write cf_mods.json: {e}")).as_error()
    })
}

// ── Поиск ─────────────────────────────────────────────────────────────────

fn loader_type(loader: &str) -> u8 {
    match loader.to_lowercase().as_str() {
        "forge" => 1,
        "fabric" => 4,
        "quilt" => 5,
        "neoforge" => 6,
        _ => 0,
    }
}

fn sort_field_id(field: &str) -> u8 {
    match field {
        "popularity" => 2,
        "lastUpdated" => 3,
        "name" => 4,
        "rating" => 5,
        "totalDownloads" => 6,
        "newest" => 11,
        _ => 2,
    }
}

fn build_search_key(
    query: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
    class_id: Option<u32>,
    sort_field: &str,
    sort_order: &str,
    page: u32,
    category_id: Option<u32>,
) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}|{}|{}|{}|{:?}",
        query, game_version, loader, class_id, sort_field, sort_order, page, category_id
    )
}

#[instrument]
pub async fn search(
    query: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
    class_id: Option<u32>,
    sort_field: Option<&str>,
    sort_order: Option<&str>,
    page: u32,
    category_id: Option<u32>,
) -> crate::Result<CfSearchResult> {
    let sf = sort_field_id(sort_field.unwrap_or("popularity")).to_string();
    let so = sort_order.unwrap_or("desc").to_string();
    let cid = class_id.unwrap_or(6).to_string();
    let key = build_search_key(query, game_version, loader, class_id, &sf, &so, page, category_id);

    // Проверяем кеш
    {
        let cache = CACHE.search.lock().unwrap();
        if let Some(entry) = cache.get(&key) {
            if CfCache::is_valid(entry) {
                tracing::debug!("CF search cache hit: {}", key);
                return Ok(entry.data.clone());
            }
        }
    }

    let client = cf_client();
    let mut req = client
        .get(format!("{CF_BASE}/v1/mods/search"))
        .query(&[
            ("gameId", MINECRAFT_GAME_ID.to_string()),
            ("classId", cid),
            ("searchFilter", query.to_string()),
            ("pageSize", "20".to_string()),
            ("index", (page * 20).to_string()),
            ("sortField", sf),
            ("sortOrder", so),
        ]);

    if let Some(gv) = game_version {
        req = req.query(&[("gameVersion", gv)]);
    }
    if let Some(l) = loader {
        let lt = loader_type(l);
        if lt > 0 {
            req = req.query(&[("modLoaderType", lt.to_string())]);
        }
    }
    if let Some(cat_id) = category_id {
        req = req.query(&[("categoryId", cat_id.to_string())]);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        tracing::error!("CurseForge API error: status={}, body={}", status, body);
        return Err(crate::ErrorKind::OtherError(format!("HTTP {}: {}", status, body)).as_error());
    }

    let result: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| crate::ErrorKind::OtherError(format!("JSON parse error: {}", e)).as_error())?;

    let search_result = serde_json::from_value(result["data"].clone()).map(|data| CfSearchResult {
        data,
        pagination: serde_json::from_value(result["pagination"].clone())
            .unwrap_or(CfPagination {
                index: page * 20,
                page_size: 20,
                result_count: 0,
                total_count: 0,
            }),
    })?;

    // Сохраняем в кеш
    {
        let mut cache = CACHE.search.lock().unwrap();
        cache.insert(key, CacheEntry {
            data: search_result.clone(),
            timestamp: Instant::now(),
        });
    }

    Ok(search_result)
}

// ── Получение мода с деталями ─────────────────────────────────────────────

#[instrument]
pub async fn get_mod(mod_id: u32) -> crate::Result<CfMod> {
    // Проверяем кеш
    {
        let cache = CACHE.mods.lock().unwrap();
        if let Some(entry) = cache.get(&mod_id) {
            if CfCache::is_valid(entry) {
                tracing::debug!("CF mod cache hit: {}", mod_id);
                return Ok(entry.data.clone());
            }
        }
    }

    let client = cf_client();
    let resp = client
        .get(format!("{CF_BASE}/v1/mods/{mod_id}"))
        .send()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        tracing::error!("CurseForge API error: status={}, body={}", status, body);
        return Err(crate::ErrorKind::OtherError(format!("HTTP {}: {}", status, body)).as_error());
    }

    let result: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| crate::ErrorKind::OtherError(format!("JSON parse error: {}", e)).as_error())?;

    let mod_data: CfMod = serde_json::from_value(result["data"].clone())?;

    // Сохраняем в кеш
    {
        let mut cache = CACHE.mods.lock().unwrap();
        cache.insert(mod_id, CacheEntry {
            data: mod_data.clone(),
            timestamp: Instant::now(),
        });
    }

    Ok(mod_data)
}

#[instrument]
pub async fn get_mod_details(mod_id: u32) -> crate::Result<CfModDetails> {
    // Проверяем кеш
    {
        let cache = CACHE.details.lock().unwrap();
        if let Some(entry) = cache.get(&mod_id) {
            if CfCache::is_valid(entry) {
                tracing::debug!("CF details cache hit: {}", mod_id);
                return Ok(entry.data.clone());
            }
        }
    }

    let mod_data = get_mod(mod_id).await?;
    let description = get_mod_description(mod_id).await?;
    let screenshots = get_mod_screenshots(mod_id).await?;

    let details = CfModDetails {
        mod_data,
        description,
        screenshots,
    };

    // Сохраняем в кеш
    {
        let mut cache = CACHE.details.lock().unwrap();
        cache.insert(mod_id, CacheEntry {
            data: details.clone(),
            timestamp: Instant::now(),
        });
    }

    Ok(details)
}

#[instrument]
pub async fn get_mod_screenshots(mod_id: u32) -> crate::Result<Vec<CfScreenshot>> {
    let client = cf_client();
    let resp = client
        .get(format!("{CF_BASE}/v1/mods/{mod_id}/screenshots"))
        .send()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        tracing::warn!("Failed to fetch screenshots: status={}, body={}", status, body);
        return Ok(Vec::new());
    }

    let result: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| crate::ErrorKind::OtherError(format!("JSON parse error: {}", e)).as_error())?;

    Ok(serde_json::from_value(result["data"].clone())?)
}

#[instrument]
pub async fn get_mod_description(mod_id: u32) -> crate::Result<String> {
    let client = cf_client();
    let resp = client
        .get(format!("{CF_BASE}/v1/mods/{mod_id}/description"))
        .send()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?
        .error_for_status()
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    Ok(resp["data"].as_str().unwrap_or("").to_string())
}

// ── Файлы и версии ─────────────────────────────────────────────────────────

#[instrument]
pub async fn get_files(
    mod_id: u32,
    game_version: Option<&str>,
    loader: Option<&str>,
) -> crate::Result<Vec<CfFile>> {
    let client = cf_client();
    let mut req = client.get(format!("{CF_BASE}/v1/mods/{mod_id}/files"));
    if let Some(gv) = game_version {
        req = req.query(&[("gameVersion", gv)]);
    }
    if let Some(l) = loader {
        let lt = loader_type(l);
        if lt > 0 {
            req = req.query(&[("modLoaderType", lt.to_string())]);
        }
    }

    let resp = req
        .send()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        tracing::error!("CurseForge API error: status={}, body={}", status, body);
        return Err(crate::ErrorKind::OtherError(format!("HTTP {}: {}", status, body)).as_error());
    }

    let result: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| crate::ErrorKind::OtherError(format!("JSON parse error: {}", e)).as_error())?;

    Ok(serde_json::from_value(result["data"].clone())?)
}

async fn resolve_best_file(
    mod_id: u32,
    game_version: &str,
    loader: &str,
) -> crate::Result<CfFile> {
    let files = get_files(mod_id, Some(game_version), Some(loader)).await?;
    files
        .into_iter()
        .find(|f| f.game_versions.iter().any(|gv| gv == game_version))
        .ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "No CurseForge file found for mod {mod_id} on {game_version} {loader}"
            ))
            .as_error()
        })
}

async fn get_file(mod_id: u32, file_id: u32) -> crate::Result<CfFile> {
    let client = cf_client();
    let resp = client
        .get(format!("{CF_BASE}/v1/mods/{mod_id}/files/{file_id}"))
        .send()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        tracing::error!("CurseForge API error: status={}, body={}", status, body);
        return Err(crate::ErrorKind::OtherError(format!("HTTP {}: {}", status, body)).as_error());
    }

    let result: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| crate::ErrorKind::OtherError(format!("JSON parse error: {}", e)).as_error())?;

    Ok(serde_json::from_value(result["data"].clone())?)
}

// ── Установка мода ─────────────────────────────────────────────────────────

#[instrument]
pub async fn install_mod(
    profile_path: &str,
    mod_id: u32,
    file_id: Option<u32>,
    game_version: &str,
    loader: &str,
) -> crate::Result<Vec<u32>> {
    let mut installed_ids: Vec<u32> = Vec::new();
    let mut queue: Vec<u32> = vec![mod_id];
    let mut visited: HashSet<u32> = HashSet::new();
    let mut sidecar = read_sidecar(profile_path).await;

    while let Some(current_mod_id) = queue.pop() {
        if visited.contains(&current_mod_id) {
            continue;
        }
        visited.insert(current_mod_id);

        let cf_mod = get_mod(current_mod_id).await?;
        let file = if current_mod_id == mod_id {
            if let Some(fid) = file_id {
                get_file(current_mod_id, fid).await?
            } else {
                resolve_best_file(current_mod_id, game_version, loader).await?
            }
        } else {
            resolve_best_file(current_mod_id, game_version, loader).await?
        };

        let download_url = file.download_url.as_deref().ok_or_else(|| {
            crate::ErrorKind::OtherError(format!(
                "CurseForge file {} has no download URL",
                file.id
            ))
            .as_error()
        })?;

        let bytes = reqwest::get(download_url)
            .await
            .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?
            .bytes()
            .await
            .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

        let hash = sha1_async(bytes.clone()).await?;

        let profile_base = crate::api::instance::get_full_path(profile_path).await?;
        let mods_dir = profile_base.join("mods");
        tokio::fs::create_dir_all(&mods_dir).await.map_err(|e| {
            crate::ErrorKind::OtherError(format!("mods dir: {e}")).as_error()
        })?;
        let dest = mods_dir.join(&file.file_name);
        tokio::fs::write(&dest, &bytes).await.map_err(|e| {
            crate::ErrorKind::OtherError(format!("write mod file: {e}")).as_error()
        })?;

        let meta = CfInstalledMeta {
            mod_id: current_mod_id,
            file_id: file.id,
            title: cf_mod.name.clone(),
            icon_url: cf_mod.logo.as_ref().map(|l| l.thumbnail_url.clone().unwrap_or(l.url.clone())),
            file_name: file.file_name.clone(),
            version: file.display_name.clone(),
            author: cf_mod.authors.first().map(|a| a.name.clone()).unwrap_or_default(),
            game_version: game_version.to_string(),
        };
        sidecar.0.insert(hash, meta);

        installed_ids.push(current_mod_id);

        for dep in &file.dependencies {
            if dep.relation_type == 3 && !visited.contains(&dep.mod_id) {
                queue.push(dep.mod_id);
            }
        }
    }

    write_sidecar(profile_path, &sidecar).await?;

    Ok(installed_ids)
}

// ── Модпаки ────────────────────────────────────────────────────────────────

#[instrument]
pub async fn install_modpack(
    mod_id: u32,
    file_id: Option<u32>,
    game_version: &str,
    loader: &str,
) -> crate::Result<String> {
    use uuid::Uuid;

    let cf_mod = get_mod(mod_id).await?;
    let file = if let Some(fid) = file_id {
        get_file(mod_id, fid).await?
    } else {
        resolve_best_file(mod_id, game_version, loader).await?
    };

    let download_url = file.download_url.as_deref().ok_or_else(|| {
        crate::ErrorKind::OtherError(format!("CF modpack file {} has no download URL", file.id))
            .as_error()
    })?;

    let zip_bytes = reqwest::get(download_url)
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?
        .bytes()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    let profile_name = format!("{} ({})", cf_mod.name, Uuid::new_v4().simple().to_string()[..8].to_string());

    let loader_mod = match loader.to_lowercase().as_str() {
        "forge" => crate::prelude::ModLoader::Forge,
        "fabric" => crate::prelude::ModLoader::Fabric,
        "quilt" => crate::prelude::ModLoader::Quilt,
        "neoforge" => crate::prelude::ModLoader::NeoForge,
        "vanilla" => crate::prelude::ModLoader::Vanilla,
        _ => crate::prelude::ModLoader::Vanilla,
    };

    let instance = crate::api::instance::create(
        profile_name,
        game_version.to_string(),
        loader_mod,
        None,
        None,
        crate::state::InstanceLink::Unmanaged,
    )
    .await?;
    let profile_path = instance.instance.id;

    if let Some(logo) = &cf_mod.logo {
        let icon_url = logo.thumbnail_url.as_deref().unwrap_or(&logo.url);
        if let Ok(icon_bytes) = reqwest::get(icon_url).await {
            if let Ok(bytes) = icon_bytes.bytes().await {
                let filename = icon_url.rsplit('/').next().unwrap_or("icon.png");
                let state = State::get().await?;
                if let Ok(icon_path) = crate::util::fetch::write_cached_icon(
                    filename,
                    &state.directories.caches_dir(),
                    bytes,
                    &state.io_semaphore,
                )
                .await
                {
                    let _ = crate::api::instance::edit(
                        &profile_path,
                        crate::state::EditInstance {
                            icon_path: Some(Some(
                                icon_path.to_string_lossy().to_string(),
                            )),
                            ..Default::default()
                        },
                    )
                    .await;
                }
            }
        }
    }

    let reader = std::io::Cursor::new(&zip_bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    if let Ok(mut manifest_file) = archive.by_name("manifest.json") {
        use std::io::Read;
        let mut manifest_str = String::new();
        manifest_file.read_to_string(&mut manifest_str).map_err(|e| {
            crate::ErrorKind::OtherError(e.to_string()).as_error()
        })?;
        drop(manifest_file);

        #[derive(serde::Deserialize)]
        struct CfManifest {
            files: Vec<CfManifestFile>,
        }
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct CfManifestFile {
            project_id: u32,
            file_id: u32,
        }

        if let Ok(manifest) = serde_json::from_str::<CfManifest>(&manifest_str) {
            let total = manifest.files.len();
            let mut failed = 0usize;
            for mf in manifest.files {
                if let Err(e) = install_mod(
                    &profile_path,
                    mf.project_id,
                    Some(mf.file_id),
                    game_version,
                    loader,
                )
                .await
                {
                    failed += 1;
                    tracing::error!(
                        "Modpack install: failed to install mod {} (file {}): {}",
                        mf.project_id,
                        mf.file_id,
                        e
                    );
                }
            }
            if failed > 0 {
                tracing::warn!(
                    "Modpack install: {failed}/{total} mods from manifest failed to install"
                );
            }
        }
    }

    for i in 0..archive.len() {
        if let Ok(mut zf) = archive.by_index(i) {
            let name = zf.name().to_string();
            if (name.starts_with("overrides/") || name.starts_with("client-overrides/"))
                && !name.ends_with('/')
            {
                let rel = name
                    .strip_prefix("overrides/")
                    .or_else(|| name.strip_prefix("client-overrides/"))
                    .unwrap_or(&name);
                let profile_base = crate::api::instance::get_full_path(&profile_path).await?;
                let dest = profile_base.join(rel);
                if let Some(parent) = dest.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                let mut buf = Vec::new();
                use std::io::Read;
                let _ = zf.read_to_end(&mut buf);
                let _ = tokio::fs::write(&dest, &buf).await;
            }
        }
    }

    Ok(profile_path)
}

// ── Категории ──────────────────────────────────────────────────────────────

#[instrument]
pub async fn get_categories(class_id: Option<u32>) -> crate::Result<Vec<CfCategory>> {
    // Проверяем кеш
    {
        let cache = CACHE.categories.lock().unwrap();
        if let Some(entry) = cache.as_ref() {
            if CfCache::is_valid(entry) {
                tracing::debug!("CF categories cache hit");
                return Ok(entry.data.clone());
            }
        }
    }

    let client = cf_client();
    let mut req = client
        .get(format!("{CF_BASE}/v1/categories"))
        .query(&[("gameId", MINECRAFT_GAME_ID.to_string())]);

    if let Some(cid) = class_id {
        req = req.query(&[("classId", cid.to_string())]);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        tracing::error!("CurseForge API error: status={}, body={}", status, body);
        return Err(crate::ErrorKind::OtherError(format!("HTTP {}: {}", status, body)).as_error());
    }

    let result: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| crate::ErrorKind::OtherError(format!("JSON parse error: {}", e)).as_error())?;

    let categories: Vec<CfCategory> = serde_json::from_value(result["data"].clone())?;

    // Сохраняем в кеш
    {
        let mut cache = CACHE.categories.lock().unwrap();
        *cache = Some(CacheEntry {
            data: categories.clone(),
            timestamp: Instant::now(),
        });
    }

    Ok(categories)
}
