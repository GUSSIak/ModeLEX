//! ModLEX: experimental workaround for a vanilla-client quirk (confirmed on
//! 1.16.5) where the Multiplayer button gets disabled for offline/Ely.by
//! accounts specifically when Mojang's real session-validation endpoints
//! are reachable during launch. Manually toggling internet off then back on
//! right after launch is a known user workaround (see the settings toggle
//! this module backs, `Settings::modlex_experimental_offline_multiplayer_fix`)
//! — this automates the "off" part for a short window around launch by
//! redirecting the relevant Mojang hostnames to localhost via the hosts
//! file, then reverting.
//!
//! An earlier attempt used `-Dhttp.proxyHost`/`-Dhttps.proxyHost` JVM
//! properties instead — confirmed live not to work, almost certainly
//! because authlib's HTTP client (era-appropriate for 1.16.5) doesn't honor
//! passive JVM proxy properties. A hosts-file redirect works regardless of
//! which HTTP client the game uses, since it operates at DNS resolution.
//!
//! Windows-only: editing the hosts file needs the same elevation as any
//! other write under `%SystemRoot%\System32` — that's a property of
//! *whichever process performs the edit*, which here is this launcher
//! itself, not the spawned Java/game process. Java itself needs no special
//! privileges merely to make HTTP requests.

#[cfg(windows)]
mod imp {
    use std::io::ErrorKind;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    use tokio::sync::Mutex;

    const MARKER_START: &str = "# ModLEX-offline-multiplayer-fix-start";
    const MARKER_END: &str = "# ModLEX-offline-multiplayer-fix-end";
    const REDIRECT_HOSTS: &[&str] = &[
        "authserver.mojang.com",
        "sessionserver.mojang.com",
        "api.mojang.com",
        "api.minecraftservices.com",
    ];

    static ACTIVE_COUNT: AtomicU32 = AtomicU32::new(0);
    static HOSTS_LOCK: Mutex<()> = Mutex::const_new(());

    fn hosts_path() -> PathBuf {
        let system_root = std::env::var_os("SystemRoot")
            .unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(system_root).join("System32\\drivers\\etc\\hosts")
    }

    /// Where a one-time pristine copy of the hosts file is kept, taken before
    /// this feature ever touches it for the first time — a safety net for
    /// "something went catastrophically wrong" independent of the normal
    /// marker-based revert, and what the manual "Restore hosts file" button
    /// in Settings restores from.
    async fn backup_path() -> crate::Result<PathBuf> {
        let state = crate::State::get().await?;
        Ok(state.directories.settings_dir.join("modlex_hosts_backup.txt"))
    }

    async fn ensure_backup_exists() {
        let Ok(path) = backup_path().await else {
            return;
        };
        if tokio::fs::try_exists(&path).await.unwrap_or(false) {
            return;
        }
        let Ok(current) = tokio::fs::read_to_string(hosts_path()).await else {
            return;
        };
        // Never back up a copy that already has our own block in it — this
        // should be unreachable in practice (begin()/end() keep the file
        // clean outside their own brief window) but a pristine backup is the
        // whole point, so guard it explicitly rather than trust that.
        let clean = strip_existing_block(&current);
        if let Err(error) = tokio::fs::write(&path, clean).await {
            tracing::warn!(
                "Failed to save a pristine hosts file backup for the offline-multiplayer fix: {error}"
            );
        }
    }

    /// Manually restores the hosts file from the one-time pristine backup,
    /// discarding whatever is currently there (including our own redirect
    /// block, if somehow still present). For the "Restore hosts file" button
    /// — a deliberate escape hatch independent of the normal begin()/end()
    /// bookkeeping, for when a user suspects something's actually wrong.
    pub async fn restore_from_backup() -> crate::Result<bool> {
        let path = backup_path().await?;
        let Ok(backup) = tokio::fs::read_to_string(&path).await else {
            return Ok(false);
        };
        let _guard = HOSTS_LOCK.lock().await;
        tokio::fs::write(hosts_path(), backup).await.map_err(|source| {
            crate::ErrorKind::FSError(format!(
                "Failed to restore hosts file from backup: {source}"
            ))
        })?;
        ACTIVE_COUNT.store(0, Ordering::SeqCst);
        Ok(true)
    }

    /// There's no clean way to ask "am I elevated" that isn't itself a chunk
    /// of Win32 API surface — but what we actually care about is narrower
    /// and more direct: can this process write to the hosts file. Probing
    /// that directly (open for append, write nothing, close) is simpler and
    /// can't give a false positive/negative the way a token-elevation check
    /// disconnected from the actual operation could.
    pub async fn can_write_hosts_file() -> bool {
        use tokio::io::AsyncWriteExt;

        let path = hosts_path();
        match tokio::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .await
        {
            Ok(mut file) => file.flush().await.is_ok(),
            Err(error) => error.kind() != ErrorKind::PermissionDenied,
        }
    }

    fn strip_existing_block(content: &str) -> String {
        let mut out = String::with_capacity(content.len());
        let mut skipping = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == MARKER_START {
                skipping = true;
                continue;
            }
            if trimmed == MARKER_END {
                skipping = false;
                continue;
            }
            if skipping {
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    async fn write_redirect_block(active: bool) -> crate::Result<()> {
        let path = hosts_path();
        let existing = tokio::fs::read_to_string(&path).await.unwrap_or_default();
        let mut cleaned = strip_existing_block(&existing);

        if active {
            cleaned.push_str(MARKER_START);
            cleaned.push('\n');
            for host in REDIRECT_HOSTS {
                cleaned.push_str(&format!("127.0.0.1 {host}\n"));
            }
            cleaned.push_str(MARKER_END);
            cleaned.push('\n');
        }

        tokio::fs::write(&path, cleaned).await.map_err(|source| {
            crate::ErrorKind::FSError(format!(
                "Failed to update hosts file for the offline-multiplayer fix: {source}"
            ))
        })?;
        Ok(())
    }

    /// Begins a redirect window. Ref-counted so concurrent launches (this
    /// launcher supports running several accounts at once) don't stomp on
    /// each other — the hosts file is only actually touched on the 0->1
    /// and 1->0 transitions.
    pub async fn begin() -> crate::Result<()> {
        ensure_backup_exists().await;

        let _guard = HOSTS_LOCK.lock().await;
        let previous = ACTIVE_COUNT.fetch_add(1, Ordering::SeqCst);
        if previous == 0 {
            write_redirect_block(true).await?;
        }
        Ok(())
    }

    pub async fn end() {
        let _guard = HOSTS_LOCK.lock().await;
        let previous = ACTIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
        if previous == 1
            && let Err(error) = write_redirect_block(false).await
        {
            tracing::warn!(
                "Failed to revert the offline-multiplayer-fix hosts redirect: {error}"
            );
        }
    }

    /// Removes any leftover redirect block from a previous run that crashed
    /// or was killed before it could clean up after itself. Call once at
    /// app startup, before anything could plausibly call `begin()`.
    pub async fn cleanup_stale_block_on_startup() {
        let path = hosts_path();
        let Ok(existing) = tokio::fs::read_to_string(&path).await else {
            return;
        };
        if !existing.contains(MARKER_START) {
            return;
        }
        let cleaned = strip_existing_block(&existing);
        match tokio::fs::write(&path, cleaned).await {
            Ok(()) => tracing::info!(
                "Cleaned up a leftover offline-multiplayer-fix hosts redirect from a previous run"
            ),
            Err(error) => tracing::warn!(
                "Found a leftover offline-multiplayer-fix hosts redirect from a previous run but failed to clean it up: {error}"
            ),
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub async fn can_write_hosts_file() -> bool {
        false
    }

    pub async fn begin() -> crate::Result<()> {
        Err(crate::ErrorKind::OtherError(
            "The offline-multiplayer fix is only implemented on Windows".to_string(),
        )
        .into())
    }

    pub async fn end() {}

    pub async fn cleanup_stale_block_on_startup() {}

    pub async fn restore_from_backup() -> crate::Result<bool> {
        Ok(false)
    }
}

pub use imp::*;
