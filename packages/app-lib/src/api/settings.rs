//! Theseus settings management interface

pub use crate::{
    State,
    state::{Hooks, MemorySettings, Settings, WindowSize},
};

/// Gets entire settings
#[tracing::instrument]
pub async fn get() -> crate::Result<Settings> {
    let state = State::get().await?;
    let settings = Settings::get(&state.pool).await?;
    Ok(settings)
}

/// Sets entire settings
#[tracing::instrument]
pub async fn set(settings: Settings) -> crate::Result<()> {
    let state = State::get().await?;
    settings.update(&state.pool).await?;

    Ok(())
}

/// Whether this process currently has permission to edit the hosts file —
/// used by the "for experienced users" settings tab to show accurate status
/// for `modlex_experimental_offline_multiplayer_fix` before the user even
/// tries to launch anything.
#[tracing::instrument]
pub async fn modlex_can_write_hosts_file() -> bool {
    crate::launcher::offline_multiplayer_fix::can_write_hosts_file().await
}

/// Manually restores the hosts file from the one-time pristine backup taken
/// the first time the offline-multiplayer fix ever ran, discarding whatever
/// is currently there. Returns `false` if no backup exists yet (the fix has
/// never actually run) rather than an error, since "nothing to restore" is
/// an expected, harmless case for this button.
#[tracing::instrument]
pub async fn modlex_restore_hosts_file() -> crate::Result<bool> {
    crate::launcher::offline_multiplayer_fix::restore_from_backup().await
}

#[tracing::instrument]
pub async fn cancel_directory_change(
    app_identifier: &str,
) -> crate::Result<()> {
    // This is called to handle state initialization errors due to folder migrations
    // failing, so fetching a DB connection pool from `State::get` is not reliable here
    let pool = crate::state::db::connect(app_identifier).await?;
    let mut settings = Settings::get(&pool).await?;

    if let Some(prev_custom_dir) = settings.prev_custom_dir {
        settings.prev_custom_dir = None;
        settings.custom_dir = Some(prev_custom_dir);
    }

    settings.update(&pool).await?;

    Ok(())
}
