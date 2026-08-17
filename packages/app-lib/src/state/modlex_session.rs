//! Exclusivity/session handshake with the companion "ModLEX Core" mod.
//!
//! Writes a small HMAC-signed JSON file that the mod reads at client startup (via
//! `-Dmodlex.session=<path>`) to verify the game was launched by ModLEX, and to receive
//! the user's configured music sources (VK token pasted in by advanced users, SoundCloud
//! toggle, local music folder). See `ModLEX Core/docs/EXCLUSIVE.md` for the mod-side
//! protocol this mirrors; `SHARED_SECRET` must match the value baked into the mod.

use hmac::{Hmac, Mac};
use rand::Rng;
use rand::rngs::OsRng;
use serde::Serialize;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::state::{ModlexPlaylist, Settings};

const SHARED_SECRET: &str = env!("MODLEX_SESSION_SECRET");
const LAUNCHER_NAME: &str = "ModLEX";

/// How long after launch the session file is deleted from disk. Generous on purpose: a
/// cold launch of a heavy modpack can take well over a minute to reach mod init, and the
/// mod reads this file during its own client-init phase. The mod itself also enforces a
/// 6h TTL on `issuedAt`, so this delay is just tidy-up, not the security boundary — a
/// late delete only means the file lingers in the temp dir a bit longer, nothing worse.
const CLEANUP_DELAY: Duration = Duration::from_secs(90);

#[derive(Serialize)]
struct ModlexSession {
    launcher: &'static str,
    launcher_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    vk_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    vk_user_id: Option<i64>,
    issued_at: i64,
    nonce: String,
    sig: String,
    soundcloud_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    local_music_path: Option<String>,
    music_default_source: String,
    playlists: Vec<ModlexPlaylist>,
}

fn canonical(
    launcher_version: &str,
    vk_token: &str,
    vk_user_id: i64,
    issued_at: i64,
    nonce: &str,
) -> String {
    format!(
        "{LAUNCHER_NAME}|{launcher_version}|{vk_token}|{vk_user_id}|{issued_at}|{nonce}"
    )
}

/// Writes the signed session file for this launch to a temp directory and returns its
/// path, suitable for passing straight into `-Dmodlex.session=<path>`.
///
/// `launcher_version` should be the user-facing app version (`State::launcher_version`,
/// sourced from `tauri.conf.json` at runtime) — NOT this crate's own `CARGO_PKG_VERSION`,
/// which is an internal library version unrelated to the shipped app version.
pub fn write_session_file(
    settings: &Settings,
    launcher_version: &str,
) -> std::io::Result<PathBuf> {
    let launcher_version = launcher_version.to_string();
    let issued_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64;

    let nonce_bytes: [u8; 16] = OsRng.r#gen();
    let nonce = hex::encode(nonce_bytes);

    let vk_token = settings.modlex_vk_token.clone().unwrap_or_default();
    let vk_user_id = settings.modlex_vk_user_id.unwrap_or_default();

    let canonical_str =
        canonical(&launcher_version, &vk_token, vk_user_id, issued_at, &nonce);
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(SHARED_SECRET.as_bytes())
        .expect("HMAC accepts a key of any length");
    mac.update(canonical_str.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());

    let session = ModlexSession {
        launcher: LAUNCHER_NAME,
        launcher_version,
        vk_token: settings.modlex_vk_token.clone(),
        vk_user_id: settings.modlex_vk_user_id,
        issued_at,
        nonce,
        sig,
        soundcloud_enabled: settings.modlex_soundcloud_enabled,
        local_music_path: settings.modlex_local_music_path.clone(),
        music_default_source: settings.modlex_music_default_source.clone(),
        playlists: settings.modlex_playlists.clone(),
    };

    let path = std::env::temp_dir().join(format!(
        "modlex_session_{}.json",
        session.nonce
    ));
    std::fs::write(
        &path,
        serde_json::to_string(&session)
            .expect("ModlexSession always serializes"),
    )?;

    Ok(path)
}

/// Deletes the session file a short while after launch. Best-effort: a failure here just
/// means the file lingers in the temp directory until the OS or the user cleans it up.
pub fn schedule_cleanup(path: PathBuf) {
    tokio::spawn(async move {
        tokio::time::sleep(CLEANUP_DELAY).await;
        if let Err(e) = tokio::fs::remove_file(&path).await {
            tracing::debug!(
                "Failed to clean up ModLEX session file {}: {e}",
                path.display()
            );
        }
    });
}
