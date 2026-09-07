//! packages/app-lib/src/api/curseforge.rs
//! CurseForge API client + install logic

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::State;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::install::{
    InstallPhaseDetails, InstallPhaseId, InstallProgress,
    InstallProgressReporter,
};
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
    // Ключ — classId (0 = "все категории", т.е. запрос без classId). Раньше это
    // был один общий слот на все classId, из-за чего запрос категорий модпаков
    // затирал в кеше категории модов (и наоборот) до истечения TTL.
    categories: Arc<Mutex<HashMap<u32, CacheEntry<Vec<CfCategory>>>>>,
    mods: Arc<Mutex<HashMap<u32, CacheEntry<CfMod>>>>,
    details: Arc<Mutex<HashMap<u32, CacheEntry<CfModDetails>>>>,
    search: Arc<Mutex<HashMap<String, CacheEntry<CfSearchResult>>>>,
}

impl CfCache {
    pub fn new() -> Self {
        Self {
            categories: Arc::new(Mutex::new(HashMap::new())),
            mods: Arc::new(Mutex::new(HashMap::new())),
            details: Arc::new(Mutex::new(HashMap::new())),
            search: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn is_valid<T>(entry: &CacheEntry<T>) -> bool {
        entry.timestamp.elapsed() < CACHE_TTL
    }

    pub fn clear(&self) {
        self.categories.lock().unwrap().clear();
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

// Встраивается во время сборки из packages/app-lib/.env (gitignored, как и
// MODLEX_SESSION_SECRET) — не хардкодим личный путь к файлу на конкретной
// машине, ключ никогда не попадает в исходники/коммиты.
const CURSEFORGE_API_KEY: &str = env!("CURSEFORGE_API_KEY");

fn get_api_key() -> &'static str {
    CURSEFORGE_API_KEY
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
    h.insert(
        "User-Agent",
        HeaderValue::from_static(
            "ModLEX/1.0 (https://github.com/GUSSIak/ModeLEX)",
        ),
    );
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

impl CfLogo {
    /// CF иногда отдаёт thumbnailUrl/url пустой строкой вместо null/пропуска
    /// поля — не то же самое, что "нет иконки".
    pub fn best_url(&self) -> Option<&str> {
        [self.thumbnail_url.as_deref(), Some(self.url.as_str())]
            .into_iter()
            .flatten()
            .find(|url| !url.is_empty())
    }
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

/// `profile_path` — уже готовый путь папки инстанции (не ID). См. memory
/// project_modlex_curseforge про path-vs-ID баги в этом файле.
pub async fn resolve_profile_dir(profile_path: &str) -> crate::Result<PathBuf> {
    let state = crate::state::State::get().await?;
    Ok(crate::util::io::canonicalize(
        state.directories.instances_dir().join(profile_path),
    )?)
}

fn sidecar_path_at(profile_base: &Path) -> PathBuf {
    profile_base.join(".modlex").join("cf_mods.json")
}

async fn read_sidecar_at(profile_base: &Path) -> CfSidecar {
    let path = sidecar_path_at(profile_base);
    let Ok(raw) = tokio::fs::read_to_string(&path).await else {
        return CfSidecar::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

async fn write_sidecar_at(
    profile_base: &Path,
    sidecar: &CfSidecar,
) -> crate::Result<()> {
    let path = sidecar_path_at(profile_base);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "Failed to create .modlex dir: {e}"
            ))
            .as_error()
        })?;
    }
    let json = serde_json::to_string_pretty(sidecar)?;
    tokio::fs::write(&path, json).await.map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Failed to write cf_mods.json: {e}"
        ))
        .as_error()
    })
}

pub async fn read_sidecar(profile_path: &str) -> CfSidecar {
    let Ok(base) = resolve_profile_dir(profile_path).await else {
        return CfSidecar::default();
    };
    read_sidecar_at(&base).await
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
        query,
        game_version,
        loader,
        class_id,
        sort_field,
        sort_order,
        page,
        category_id
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
    let key = build_search_key(
        query,
        game_version,
        loader,
        class_id,
        &sf,
        &so,
        page,
        category_id,
    );

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
    let mut req = client.get(format!("{CF_BASE}/v1/mods/search")).query(&[
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
        tracing::error!(
            "CurseForge API error: status={}, body={}",
            status,
            body
        );
        return Err(crate::ErrorKind::OtherError(format!(
            "HTTP {}: {}",
            status, body
        ))
        .as_error());
    }

    let result: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| {
            crate::ErrorKind::OtherError(format!("JSON parse error: {}", e))
                .as_error()
        })?;

    let search_result =
        serde_json::from_value(result["data"].clone()).map(|data| {
            CfSearchResult {
                data,
                pagination: serde_json::from_value(
                    result["pagination"].clone(),
                )
                .unwrap_or(CfPagination {
                    index: page * 20,
                    page_size: 20,
                    result_count: 0,
                    total_count: 0,
                }),
            }
        })?;

    // Сохраняем в кеш
    {
        let mut cache = CACHE.search.lock().unwrap();
        cache.insert(
            key,
            CacheEntry {
                data: search_result.clone(),
                timestamp: Instant::now(),
            },
        );
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
        tracing::error!(
            "CurseForge API error: status={}, body={}",
            status,
            body
        );
        return Err(crate::ErrorKind::OtherError(format!(
            "HTTP {}: {}",
            status, body
        ))
        .as_error());
    }

    let result: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| {
            crate::ErrorKind::OtherError(format!("JSON parse error: {}", e))
                .as_error()
        })?;

    let mod_data: CfMod = serde_json::from_value(result["data"].clone())?;

    // Сохраняем в кеш
    {
        let mut cache = CACHE.mods.lock().unwrap();
        cache.insert(
            mod_id,
            CacheEntry {
                data: mod_data.clone(),
                timestamp: Instant::now(),
            },
        );
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
        cache.insert(
            mod_id,
            CacheEntry {
                data: details.clone(),
                timestamp: Instant::now(),
            },
        );
    }

    Ok(details)
}

#[instrument]
pub async fn get_mod_screenshots(
    mod_id: u32,
) -> crate::Result<Vec<CfScreenshot>> {
    let client = cf_client();
    let resp = client
        .get(format!("{CF_BASE}/v1/mods/{mod_id}/screenshots"))
        .send()
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    let status = resp.status();
    let body = resp.text().await.unwrap_or_default();

    if !status.is_success() {
        tracing::warn!(
            "Failed to fetch screenshots: status={}, body={}",
            status,
            body
        );
        return Ok(Vec::new());
    }

    let result: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| {
            crate::ErrorKind::OtherError(format!("JSON parse error: {}", e))
                .as_error()
        })?;

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
        tracing::error!(
            "CurseForge API error: status={}, body={}",
            status,
            body
        );
        return Err(crate::ErrorKind::OtherError(format!(
            "HTTP {}: {}",
            status, body
        ))
        .as_error());
    }

    let result: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| {
            crate::ErrorKind::OtherError(format!("JSON parse error: {}", e))
                .as_error()
        })?;

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
        tracing::error!(
            "CurseForge API error: status={}, body={}",
            status,
            body
        );
        return Err(crate::ErrorKind::OtherError(format!(
            "HTTP {}: {}",
            status, body
        ))
        .as_error());
    }

    let result: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| {
            crate::ErrorKind::OtherError(format!("JSON parse error: {}", e))
                .as_error()
        })?;

    Ok(serde_json::from_value(result["data"].clone())?)
}

// ── Скачивание файлов ────────────────────────────────────────────────────────

/// CF API у части файлов отдаёт `downloadUrl: null` даже когда файл реально
/// скачиваем — это не то же самое, что мод с полностью отключённой раздачей
/// третьим лицам (`allowModDistribution: false`, тогда CDN тоже вернёт 403).
/// CDN CurseForge раскладывает файлы по предсказуемому пути `files/{id/1000}/
/// {id%1000}/{filename}` — этим же фолбэком пользуются другие сторонние CF-
/// клиенты (ferium, packwiz и т.п.), когда официальное поле пустое.
fn cf_cdn_fallback_url(file: &CfFile) -> String {
    format!(
        "https://edge.forgecdn.net/files/{}/{}/{}",
        file.id / 1000,
        file.id % 1000,
        urlencoding::encode(&file.file_name)
    )
}

/// Скачивает файл мода/модпака: использует `downloadUrl` из API, если он
/// есть, иначе пробует прямой CDN-путь. Если и это возвращает не-2xx —
/// значит автор реально отключил раздачу через API, и это сообщаем понятно,
/// а не сырой сетевой ошибкой.
/// CurseForge не всегда честно отдаёт Content-Length, а modpack-архив может
/// быть заявлен маленьким и раздуться при реальной докачке — читаем чанками
/// и обрываем сразу по факту превышения лимита, а не полагаемся на
/// заголовок. 1 GiB — щедрый запас для любого реального модпака (с ресурс-
/// и шейдерпаками включительно), но не безлимит, который может забить
/// память процесса при скачивании специально раздутого/скомпрометированного
/// файла.
const MAX_MODPACK_DOWNLOAD_BYTES: usize = 1024 * 1024 * 1024;

/// Скачивает `url` с жёстким потолком размера — используется там, где ответ
/// не критичен для установки (иконки) и достаточно молча вернуть `None` на
/// любую проблему (сеть, статус, превышение лимита), а не обрывать всю
/// установку модпака из-за недоступной картинки.
async fn download_bounded(url: &str, max_bytes: usize) -> Option<bytes::Bytes> {
    let response = reqwest::get(url).await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    if response.content_length().is_some_and(|len| len > max_bytes as u64) {
        return None;
    }

    use futures::StreamExt;
    let mut stream = response.bytes_stream();
    let mut buf = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.ok()?;
        if buf.len() + chunk.len() > max_bytes {
            return None;
        }
        buf.extend_from_slice(&chunk);
    }
    Some(bytes::Bytes::from(buf))
}

async fn download_cf_file(file: &CfFile) -> crate::Result<bytes::Bytes> {
    let download_url = file
        .download_url
        .clone()
        .unwrap_or_else(|| cf_cdn_fallback_url(file));

    let response = reqwest::get(&download_url)
        .await
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    if !response.status().is_success() {
        return Err(crate::ErrorKind::OtherError(format!(
            "Файл \"{}\" недоступен для скачивания — автор отключил раздачу через сторонние \
             приложения для этого проекта на CurseForge. Скачайте его вручную с сайта \
             curseforge.com и установите файл вручную.",
            file.file_name
        ))
        .as_error());
    }

    if response.content_length().is_some_and(|len| len > MAX_MODPACK_DOWNLOAD_BYTES as u64) {
        return Err(crate::ErrorKind::OtherError(format!(
            "Файл \"{}\" превышает допустимый размер скачивания.",
            file.file_name
        ))
        .as_error());
    }

    use futures::StreamExt;
    let mut stream = response.bytes_stream();
    let mut buf = Vec::new();
    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;
        if buf.len() + chunk.len() > MAX_MODPACK_DOWNLOAD_BYTES {
            return Err(crate::ErrorKind::OtherError(format!(
                "Файл \"{}\" превысил допустимый размер при скачивании.",
                file.file_name
            ))
            .as_error());
        }
        buf.extend_from_slice(&chunk);
    }
    let bytes = bytes::Bytes::from(buf);

    // CurseForge отдаёт SHA-1 как algo == 1 в списке hashes файла — сверяем,
    // если он есть (не у всех файлов он проставлен), чтобы обнаружить
    // подмену/повреждение на пути от CDN, а не только удобно посчитать хеш
    // для внутреннего учёта (как было раньше — считался, но ни с чем не
    // сравнивался).
    if let Some(expected) = file
        .hashes
        .iter()
        .find(|h| h.algo == 1)
        .map(|h| h.value.to_lowercase())
    {
        let actual = sha1_async(bytes.clone()).await?;
        if actual.to_lowercase() != expected {
            return Err(crate::ErrorKind::OtherError(format!(
                "Файл \"{}\" не прошёл проверку целостности (SHA-1 не совпадает с ожидаемым) — скачивание отменено.",
                file.file_name
            ))
            .as_error());
        }
    }

    Ok(bytes)
}

// ── Установка мода ─────────────────────────────────────────────────────────

/// CurseForge classId constants for Minecraft content — mirrors
/// `CF_CLASS_IDS` in apps/app-frontend/src/helpers/curseforge.ts. Keep both
/// in sync if CF ever adds a type we care about.
const CF_CLASS_RESOURCEPACK: u32 = 12;
const CF_CLASS_SHADER: u32 = 6552;
const CF_CLASS_DATAPACK: u32 = 6945;
const CF_CLASS_WORLD: u32 = 17;

/// Where a downloaded CF file actually belongs, based on the project's
/// classId. Before this, EVERYTHING landed in `mods/` unconditionally —
/// silently wrong for anything that isn't a mod: Minecraft only scans
/// `resourcepacks/`/`shaderpacks/` for those, so a resourcepack "installed"
/// into `mods/` just never shows up in-game, with no error to explain why.
enum CfInstallTarget {
    /// Goes straight into this top-level instance folder.
    Folder(&'static str),
    /// Not handled by THIS function (`install_mod_into`) — datapacks/worlds
    /// need per-world targeting or zip extraction respectively, a different
    /// shape of operation than "drop a file in a folder". The top-level
    /// `install_mod` entry point below checks for these classIds FIRST and
    /// routes to `install_datapack`/`install_world` instead of ever reaching
    /// here — this branch only fires if one of them somehow shows up nested
    /// inside a modpack manifest (`install_mod_into`'s other caller), which
    /// CF modpacks don't realistically do. Failing loudly there beats
    /// silently dropping the file somewhere it'll never be read from.
    Unsupported(&'static str),
}

fn cf_install_target(class_id: Option<u32>) -> CfInstallTarget {
    match class_id {
        Some(CF_CLASS_RESOURCEPACK) => CfInstallTarget::Folder("resourcepacks"),
        Some(CF_CLASS_SHADER) => CfInstallTarget::Folder("shaderpacks"),
        Some(CF_CLASS_DATAPACK) => CfInstallTarget::Unsupported(
            "Датапаки CurseForge устанавливаются в конкретный мир и не могут \
             быть частью содержимого модпака.",
        ),
        Some(CF_CLASS_WORLD) => CfInstallTarget::Unsupported(
            "Миры CurseForge не могут быть частью содержимого модпака.",
        ),
        // Mods (6), modpacks (4471, installed via a different path entirely),
        // and anything unrecognized default to mods/ — unchanged prior behavior.
        _ => CfInstallTarget::Folder("mods"),
    }
}

fn sanitize_world_folder_name(input: &str) -> String {
    let cleaned = input
        .trim()
        .replace(['/', '\\', '?', '*', ':', '\'', '\"', '|', '<', '>', '!'], "_");
    if cleaned.is_empty() {
        "World".to_string()
    } else {
        cleaned
    }
}

/// Top-level CF single-project install entry point. Routes datapacks (need a
/// target world) and worlds (need zip extraction) to their own dedicated
/// functions — everything else (mods, resourcepacks, shaders) goes through
/// the shared `install_mod_into`, which also walks CF's "required dependency"
/// chain (only meaningful for mods).
#[instrument]
pub async fn install_mod(
    profile_path: &str,
    mod_id: u32,
    file_id: Option<u32>,
    game_version: &str,
    loader: &str,
    world_folder: Option<String>,
) -> crate::Result<Vec<u32>> {
    let cf_mod = get_mod(mod_id).await?;
    match cf_mod.class_id {
        Some(CF_CLASS_DATAPACK) => {
            let world_folder = world_folder.ok_or_else(|| {
                crate::ErrorKind::InputError(
                    "Нужно выбрать мир, в который установить датапак."
                        .to_string(),
                )
                .as_error()
            })?;
            install_datapack(
                profile_path,
                &world_folder,
                mod_id,
                file_id,
                game_version,
                loader,
            )
            .await?;
            Ok(vec![mod_id])
        }
        Some(CF_CLASS_WORLD) => {
            install_world(profile_path, mod_id, file_id, game_version, loader)
                .await?;
            Ok(vec![mod_id])
        }
        _ => {
            let profile_base = resolve_profile_dir(profile_path).await?;
            install_mod_into(&profile_base, mod_id, file_id, game_version, loader)
                .await
        }
    }
}

/// Installs a CurseForge datapack (classId 6945) into ONE SPECIFIC world's
/// `datapacks/` folder. Unlike mods/resourcepacks/shaders, a datapack has no
/// single "instance-wide" home — it belongs to one save — so the caller
/// (`apps/app-frontend/src/providers/cf-content-install.ts`) resolves which
/// world to target (via the existing `get_instance_worlds`, auto-picking when
/// there's exactly one) before calling this. Minecraft loads datapacks
/// straight from a zip, so — unlike worlds — no extraction is needed here.
#[instrument]
pub async fn install_datapack(
    profile_path: &str,
    world_folder: &str,
    mod_id: u32,
    file_id: Option<u32>,
    game_version: &str,
    loader: &str,
) -> crate::Result<()> {
    let profile_base = resolve_profile_dir(profile_path).await?;
    let world_dir = profile_base.join("saves").join(world_folder);
    if !world_dir.join("level.dat").exists() {
        return Err(crate::ErrorKind::InputError(format!(
            "Мир \"{world_folder}\" не найден в этой инстанции."
        ))
        .as_error());
    }

    let file = if let Some(fid) = file_id {
        get_file(mod_id, fid).await?
    } else {
        resolve_best_file(mod_id, game_version, loader).await?
    };
    let bytes = download_cf_file(&file).await?;

    let datapacks_dir = world_dir.join("datapacks");
    tokio::fs::create_dir_all(&datapacks_dir).await.map_err(|e| {
        crate::ErrorKind::OtherError(format!("datapacks dir: {e}")).as_error()
    })?;
    let dest = datapacks_dir.join(&file.file_name);
    tokio::fs::write(&dest, &bytes).await.map_err(|e| {
        crate::ErrorKind::OtherError(format!("write datapack file: {e}"))
            .as_error()
    })?;

    Ok(())
}

/// Installs a CurseForge "world" project (classId 17) — these ship as a zip
/// of a save folder, so unlike everything else here this needs real
/// extraction into `saves/`, not just dropping one file. Always creates a
/// NEW world folder (never overwrites an existing save) and returns its name.
#[instrument]
pub async fn install_world(
    profile_path: &str,
    mod_id: u32,
    file_id: Option<u32>,
    game_version: &str,
    loader: &str,
) -> crate::Result<String> {
    let profile_base = resolve_profile_dir(profile_path).await?;
    let cf_mod = get_mod(mod_id).await?;

    let file = if let Some(fid) = file_id {
        get_file(mod_id, fid).await?
    } else {
        resolve_best_file(mod_id, game_version, loader).await?
    };
    let bytes = download_cf_file(&file).await?;

    let reader = std::io::Cursor::new(&bytes);
    let mut archive = zip::ZipArchive::new(reader).map_err(|e| {
        crate::ErrorKind::OtherError(format!(
            "Не удалось прочитать архив мира: {e}"
        ))
        .as_error()
    })?;

    // CF world zips are usually one top-level folder (the save itself)
    // rather than files at the archive root — detect and strip that prefix
    // so extraction doesn't end up as saves/<name>/<name>/level.dat.
    let common_prefix: Option<String> = {
        let mut prefix: Option<String> = None;
        for i in 0..archive.len() {
            let Ok(zf) = archive.by_index(i) else {
                continue;
            };
            let name = zf.name().to_string();
            drop(zf);
            if name.ends_with('/') {
                continue;
            }
            let top = name.split('/').next().unwrap_or("").to_string();
            if top.is_empty() {
                prefix = None;
                break;
            }
            match &prefix {
                None => prefix = Some(top),
                Some(p) if *p == top => {}
                Some(_) => {
                    prefix = None;
                    break;
                }
            }
        }
        prefix
    };

    let saves_dir = profile_base.join("saves");
    tokio::fs::create_dir_all(&saves_dir).await.map_err(|e| {
        crate::ErrorKind::OtherError(format!("saves dir: {e}")).as_error()
    })?;

    // Pick a folder name that doesn't collide with an existing save.
    let base_name =
        sanitize_world_folder_name(common_prefix.as_deref().unwrap_or(&cf_mod.name));
    let mut target_name = base_name.clone();
    let mut attempt = 1;
    while saves_dir.join(&target_name).exists() {
        attempt += 1;
        target_name = format!("{base_name} ({attempt})");
    }
    let target_dir = saves_dir.join(&target_name);
    tokio::fs::create_dir_all(&target_dir).await.map_err(|e| {
        crate::ErrorKind::OtherError(format!("world dir: {e}")).as_error()
    })?;

    let mut found_level_dat = false;
    for i in 0..archive.len() {
        let Ok(mut zf) = archive.by_index(i) else {
            continue;
        };
        let name = zf.name().to_string();
        if name.ends_with('/') {
            continue;
        }
        let rel = match &common_prefix {
            Some(p) => name.strip_prefix(&format!("{p}/")).unwrap_or(&name),
            None => &name,
        };
        if rel.is_empty() {
            continue;
        }
        // Zip Slip: never trust the archive entry's own path — see
        // safe_extract_path's doc comment for why PathBuf::join alone isn't safe.
        let Some(dest) = safe_extract_path(&target_dir, rel) else {
            tracing::warn!("Skipping unsafe zip entry in CF world archive: {name}");
            continue;
        };
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                crate::ErrorKind::OtherError(format!("world extract dir: {e}"))
                    .as_error()
            })?;
        }
        use std::io::Read;
        let mut buf = Vec::new();
        zf.read_to_end(&mut buf).map_err(|e| {
            crate::ErrorKind::OtherError(format!(
                "reading world archive entry {name}: {e}"
            ))
            .as_error()
        })?;
        tokio::fs::write(&dest, &buf).await.map_err(|e| {
            crate::ErrorKind::OtherError(format!("writing world file {rel}: {e}"))
                .as_error()
        })?;
        if rel == "level.dat" {
            found_level_dat = true;
        }
    }

    if !found_level_dat {
        // An archive with no level.dat isn't a real world — leaving a
        // half-extracted, non-functional folder around under saves/ would be
        // more confusing than a clean failure, so clean it up.
        let _ = tokio::fs::remove_dir_all(&target_dir).await;
        return Err(crate::ErrorKind::OtherError(
            "Архив мира не похож на настоящий мир Minecraft (не найден level.dat) — установка отменена."
                .to_string(),
        )
        .as_error());
    }

    Ok(target_name)
}

/// Принимает уже разрешённый путь к папке инстанции напрямую (см. memory
/// project_modlex_curseforge про регрессию 2026-08-31 с manifest-циклом).
async fn install_mod_into(
    profile_base: &Path,
    mod_id: u32,
    file_id: Option<u32>,
    game_version: &str,
    loader: &str,
) -> crate::Result<Vec<u32>> {
    let mut installed_ids: Vec<u32> = Vec::new();
    let mut queue: Vec<u32> = vec![mod_id];
    let mut visited: HashSet<u32> = HashSet::new();
    let mut sidecar = read_sidecar_at(profile_base).await;

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

        let dir_name = match cf_install_target(cf_mod.class_id) {
            CfInstallTarget::Folder(name) => name,
            CfInstallTarget::Unsupported(message) => {
                return Err(crate::ErrorKind::OtherError(message.to_string()).as_error());
            }
        };

        let bytes = download_cf_file(&file).await?;

        let hash = sha1_async(bytes.clone()).await?;

        let target_dir = profile_base.join(dir_name);
        tokio::fs::create_dir_all(&target_dir).await.map_err(|e| {
            crate::ErrorKind::OtherError(format!("{dir_name} dir: {e}")).as_error()
        })?;
        let dest = target_dir.join(&file.file_name);
        tokio::fs::write(&dest, &bytes).await.map_err(|e| {
            crate::ErrorKind::OtherError(format!("write file: {e}")).as_error()
        })?;

        let meta = CfInstalledMeta {
            mod_id: current_mod_id,
            file_id: file.id,
            title: cf_mod.name.clone(),
            icon_url: cf_mod
                .logo
                .as_ref()
                .map(|l| l.thumbnail_url.clone().unwrap_or(l.url.clone())),
            file_name: file.file_name.clone(),
            version: file.display_name.clone(),
            author: cf_mod
                .authors
                .first()
                .map(|a| a.name.clone())
                .unwrap_or_default(),
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

    write_sidecar_at(profile_base, &sidecar).await?;

    Ok(installed_ids)
}

/// Безопасно резолвит путь ZIP-записи относительно `base` — построчно идёт
/// по компонентам имени и отклоняет всё, что не является обычным сегментом
/// (`..`, абсолютные пути, Windows `C:\...`, UNC `\\server\share`), вместо
/// того чтобы довериться `PathBuf::join` (её семантика join НЕ защищает от
/// этого — см. документацию `Path::join`: абсолютный путь целиком заменяет
/// базу, а `..`-компоненты в результирующем пути честно поднимаются выше
/// при реальном обращении к файлу). Возвращает `None`, если запись небезопасна
/// — вызывающий код должен её просто пропустить, а не паниковать/падать.
fn safe_extract_path(base: &Path, entry_name: &str) -> Option<PathBuf> {
    let mut resolved = base.to_path_buf();
    for component in Path::new(entry_name).components() {
        match component {
            std::path::Component::Normal(part) => resolved.push(part),
            std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    Some(resolved)
}

// ── Модпаки ────────────────────────────────────────────────────────────────

/// Устанавливает CF-модпак в уже созданный (пустой) инстанс, отчитываясь о
/// прогрессе через тот же InstallProgressReporter/InstallJob пайплайн, что и
/// обычные Modrinth-модпаки (install_mrpack.rs) — тот же top-right индикатор
/// в UI получает эти обновления бесплатно, без отдельного фронтенд-кода.
/// instance_id уже существует к моменту вызова: его создаёт
/// install::runner::prepare_initial_instance до того, как этот код вообще
/// запускается (см. get_instance_from_pack в install_from.rs).
#[instrument(skip(reporter))]
pub(crate) async fn install_modpack_with_reporter(
    mod_id: u32,
    file_id: Option<u32>,
    game_version: &str,
    loader: &str,
    instance_id: String,
    reporter: InstallProgressReporter,
) -> crate::Result<()> {
    let cf_mod = get_mod(mod_id).await?;
    let details = InstallPhaseDetails::Modpack {
        project_id: Some(mod_id.to_string()),
        version_id: file_id.map(|f| f.to_string()),
        title: Some(cf_mod.name.clone()),
    };

    // Иконки — маленькие картинки, но раз URL берётся из внешнего API без
    // проверки, ограничиваем размер разумным потолком, а не грузим что
    // угодно, что автор проекта туда подставил.
    const MAX_ICON_BYTES: usize = 10 * 1024 * 1024;

    if let Some(icon_url) = cf_mod.logo.as_ref().and_then(CfLogo::best_url) {
        if let Some(bytes) = download_bounded(icon_url, MAX_ICON_BYTES).await {
            let state = State::get().await?;
            if let Ok(icon_path) =
                crate::api::instance::cache_icon(bytes, &state).await
            {
                let _ = crate::api::instance::edit(
                    &instance_id,
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

    reporter
        .update(InstallPhaseId::DownloadingPackFile, None, details.clone())
        .await?;

    let file = if let Some(fid) = file_id {
        get_file(mod_id, fid).await?
    } else {
        resolve_best_file(mod_id, game_version, loader).await?
    };
    let zip_bytes = download_cf_file(&file).await?;

    reporter
        .update(InstallPhaseId::ReadingPackManifest, None, details.clone())
        .await?;

    let reader = std::io::Cursor::new(&zip_bytes);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| crate::ErrorKind::OtherError(e.to_string()).as_error())?;

    let profile_base = crate::api::instance::get_full_path(&instance_id).await?;

    if let Ok(mut manifest_file) = archive.by_name("manifest.json") {
        use std::io::Read;
        let mut manifest_str = String::new();
        manifest_file
            .read_to_string(&mut manifest_str)
            .map_err(|e| {
                crate::ErrorKind::OtherError(e.to_string()).as_error()
            })?;
        drop(manifest_file);

        #[derive(serde::Deserialize)]
        struct CfManifest {
            files: Vec<CfManifestFile>,
        }
        #[derive(serde::Deserialize)]
        struct CfManifestFile {
            #[serde(rename = "projectID")]
            project_id: u32,
            #[serde(rename = "fileID")]
            file_id: u32,
        }

        // Лимит на количество записей манифеста — без него специально
        // собранный (или просто битый) modpack мог бы заявить огромное
        // число файлов и запустить лавину API-запросов/установок на каждую.
        const MAX_MANIFEST_FILES: usize = 2000;

        match serde_json::from_str::<CfManifest>(&manifest_str) {
            Ok(manifest) if manifest.files.len() > MAX_MANIFEST_FILES => {
                return Err(crate::ErrorKind::OtherError(format!(
                    "Modpack содержит слишком много файлов в manifest.json ({} > {}) — установка отменена.",
                    manifest.files.len(),
                    MAX_MANIFEST_FILES
                ))
                .as_error());
            }
            Ok(manifest) => {
                let total = manifest.files.len() as u64;
                reporter
                    .update(
                        InstallPhaseId::DownloadingContent,
                        Some(InstallProgress {
                            current: 0,
                            total,
                            secondary: None,
                        }),
                        details.clone(),
                    )
                    .await?;

                let mut installed = 0u64;
                let mut failed = 0usize;
                for mf in manifest.files {
                    if let Err(e) = install_mod_into(
                        &profile_base,
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
                    installed += 1;
                    reporter
                        .update(
                            InstallPhaseId::DownloadingContent,
                            Some(InstallProgress {
                                current: installed,
                                total,
                                secondary: None,
                            }),
                            details.clone(),
                        )
                        .await?;
                }
                if failed > 0 {
                    tracing::warn!(
                        "Modpack install: {failed}/{total} mods from manifest failed to install"
                    );
                }
            }
            Err(e) => {
                tracing::error!(
                    "Modpack install: failed to parse manifest.json: {e}"
                );
            }
        }
    }

    reporter
        .update(InstallPhaseId::ExtractingOverrides, None, details.clone())
        .await?;

    for i in 0..archive.len() {
        if let Ok(mut zf) = archive.by_index(i) {
            let name = zf.name().to_string();
            if (name.starts_with("overrides/")
                || name.starts_with("client-overrides/"))
                && !name.ends_with('/')
            {
                let rel = name
                    .strip_prefix("overrides/")
                    .or_else(|| name.strip_prefix("client-overrides/"))
                    .unwrap_or(&name);
                // Zip Slip: не доверяем имени записи в архиве напрямую —
                // `PathBuf::join` либо целиком заменяет базу, если rel
                // абсолютный, либо честно поднимается через `..` куда
                // угодно на диске. Специально собранный modpack мог бы
                // записать файл за пределами профиля инстанции.
                let Some(dest) = safe_extract_path(&profile_base, rel) else {
                    tracing::warn!(
                        "Modpack install: skipping unsafe archive entry \"{name}\""
                    );
                    continue;
                };
                if let Some(parent) = dest.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|e| {
                        crate::ErrorKind::OtherError(e.to_string()).as_error()
                    })?;
                }
                let mut buf = Vec::new();
                use std::io::Read;
                zf.read_to_end(&mut buf).map_err(|e| {
                    crate::ErrorKind::OtherError(e.to_string()).as_error()
                })?;
                tokio::fs::write(&dest, &buf).await.map_err(|e| {
                    crate::ErrorKind::OtherError(e.to_string()).as_error()
                })?;
            }
        }
    }

    // Дальше — установка подходящих Minecraft+Java, тот же самый шаг с тем
    // же прогрессом (ResolvingMinecraft/DownloadingMinecraft/PreparingJava/
    // RunningLoaderProcessors), которым уже пользуются Modrinth-модпаки.
    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        &instance_id,
        false,
        Some(reporter),
    )
    .await?;

    Ok(())
}

// ── Категории ──────────────────────────────────────────────────────────────

#[instrument]
pub async fn get_categories(
    class_id: Option<u32>,
) -> crate::Result<Vec<CfCategory>> {
    let cache_key = class_id.unwrap_or(0);

    // Проверяем кеш
    {
        let cache = CACHE.categories.lock().unwrap();
        if let Some(entry) = cache.get(&cache_key) {
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
        tracing::error!(
            "CurseForge API error: status={}, body={}",
            status,
            body
        );
        return Err(crate::ErrorKind::OtherError(format!(
            "HTTP {}: {}",
            status, body
        ))
        .as_error());
    }

    let result: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| {
            crate::ErrorKind::OtherError(format!("JSON parse error: {}", e))
                .as_error()
        })?;

    let categories: Vec<CfCategory> =
        serde_json::from_value(result["data"].clone())?;

    // Сохраняем в кеш
    {
        let mut cache = CACHE.categories.lock().unwrap();
        cache.insert(
            cache_key,
            CacheEntry {
                data: categories.clone(),
                timestamp: Instant::now(),
            },
        );
    }

    Ok(categories)
}
