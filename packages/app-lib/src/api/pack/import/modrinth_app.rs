use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sqlx::sqlite::{SqliteConnectOptions, SqliteConnection};
use sqlx::ConnectOptions;

use crate::{
    State,
    install::{InstallPhaseDetails, InstallProgressReporter},
    pack::{
        import::{self, finish_import},
        install_from::{self, CreatePackDescription, PackDependency},
    },
    state::ModLoader,
};

/// ModLEX: the real Modrinth App is the exact same fork lineage as this app —
/// same `instances`/`instance_content_sets` schema — so instead of parsing a
/// foreign config format like the other importers, we read its `app.db`
/// directly (read-only; we never write to another app's database).
async fn open_read_only(db_path: &Path) -> crate::Result<SqliteConnection> {
    let conn_options = SqliteConnectOptions::new()
        .filename(db_path)
        .busy_timeout(Duration::from_secs(5))
        .read_only(true)
        .create_if_missing(false);

    Ok(conn_options.connect().await?)
}

fn db_path(base_path: &Path) -> PathBuf {
    base_path.join("app.db")
}

pub async fn get_instances(base_path: &Path) -> crate::Result<Vec<String>> {
    let mut conn = match open_read_only(&db_path(base_path)).await {
        Ok(conn) => conn,
        // Not every machine with a ModrinthApp data folder actually has an
        // app.db yet (fresh install, or an old pre-DB version) — that's not
        // an error, it just means there's nothing importable here.
        Err(_) => return Ok(Vec::new()),
    };

    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM instances ORDER BY name COLLATE NOCASE",
    )
    .fetch_all(&mut conn)
    .await
    .unwrap_or_default();

    Ok(names)
}

struct ModrinthAppInstanceRow {
    path: String,
    icon_path: Option<String>,
    game_version: Option<String>,
    loader: Option<String>,
    loader_version: Option<String>,
}

async fn get_instance_row(
    conn: &mut SqliteConnection,
    instance_name: &str,
) -> crate::Result<ModrinthAppInstanceRow> {
    let row: (String, Option<String>, Option<String>, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT i.path, i.icon_path, cs.game_version, cs.loader, cs.loader_version \
         FROM instances i \
         LEFT JOIN instance_content_sets cs ON cs.id = i.applied_content_set_id \
         WHERE i.name = ?1 \
         ORDER BY i.modified DESC LIMIT 1",
    )
    .bind(instance_name)
    .fetch_optional(conn)
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Could not find instance '{instance_name}' in the Modrinth App database"
        ))
    })?;

    Ok(ModrinthAppInstanceRow {
        path: row.0,
        icon_path: row.1,
        game_version: row.2,
        loader: row.3,
        loader_version: row.4,
    })
}

pub async fn import_modrinth_app_instance(
    base_path: PathBuf,
    instance_name: String,
    instance_id: &str,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    let mut conn = open_read_only(&db_path(&base_path)).await?;
    let row = get_instance_row(&mut conn, &instance_name).await?;
    // Close the connection promptly rather than holding it for the whole
    // (potentially slow) file copy below.
    drop(conn);

    let icon = if let Some(icon_path) = row.icon_path {
        import::recache_icon(base_path.join(icon_path)).await?
    } else {
        None
    };

    let description = CreatePackDescription {
        icon,
        override_title: Some(instance_name),
        project_id: None,
        version_id: None,
        instance_id: instance_id.to_string(),
        source_filename: None,
    };

    let mut dependencies = HashMap::new();
    if let Some(game_version) = row.game_version {
        dependencies.insert(PackDependency::Minecraft, game_version);
    }
    if let (Some(loader), Some(loader_version)) =
        (row.loader.as_deref(), row.loader_version)
    {
        let dependency = match ModLoader::from_string(loader) {
            ModLoader::Forge => Some(PackDependency::Forge),
            ModLoader::NeoForge => Some(PackDependency::NeoForge),
            ModLoader::Fabric => Some(PackDependency::FabricLoader),
            ModLoader::Quilt => Some(PackDependency::QuiltLoader),
            ModLoader::Vanilla => None,
        };
        if let Some(dependency) = dependency {
            dependencies.insert(dependency, loader_version);
        }
    }

    install_from::set_instance_information(
        instance_id.to_string(),
        &description,
        "Imported Modrinth Instance",
        None,
        &dependencies,
        false,
    )
    .await?;

    let instance_folder = base_path.join("profiles").join(row.path);
    let state = State::get().await?;
    finish_import(
        instance_id,
        instance_folder,
        &state.io_semaphore,
        reporter,
        details,
    )
    .await?;

    Ok(())
}
