//! Authentication against [Ely.by](https://ely.by), an authlib-injector-compatible
//! third-party Yggdrasil service, used as an alternative to Microsoft/Mojang accounts.
//!
//! Unlike Microsoft's login flow, Ely.by has no desktop-friendly redirect signal page,
//! so the authorization code is captured via a fixed local loopback listener
//! (see `apps/app/src/api/oauth_utils/auth_code_reply.rs`) rather than by watching a
//! webview's URL. The registered redirect_uri must match [`REDIRECT_URI`] exactly.

use super::{AccountKind, Credentials, MinecraftAuthStep, MinecraftAuthenticationError, MinecraftProfile};
use crate::util::fetch::INSECURE_REQWEST_CLIENT;
use chrono::{Duration, Utc};
use serde::Deserialize;
use std::time::Instant;
use url::Url;
use uuid::Uuid;

const CLIENT_ID: &str = env!("ELYBY_CLIENT_ID");
const CLIENT_SECRET: &str = env!("ELYBY_CLIENT_SECRET");

/// The fixed local redirect URI this app registers with Ely.by.
pub const REDIRECT_URI: &str = "http://127.0.0.1:38271";
pub const REDIRECT_PORT: u16 = 38271;

const SCOPES: &str = "account_info account_email offline_access minecraft_server_session";

#[derive(Debug)]
pub struct ElyByLoginFlow {
    pub auth_request_uri: String,
}

pub fn begin_login() -> crate::Result<ElyByLoginFlow> {
    let mut url = Url::parse("https://account.ely.by/oauth2/v1")
        .map_err(|e| crate::ErrorKind::OtherError(format!("Failed to build Ely.by authorization URL: {e}")))?;

    url.query_pairs_mut()
        .append_pair("client_id", CLIENT_ID)
        .append_pair("redirect_uri", REDIRECT_URI)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPES)
        .append_pair("prompt", "select_account");

    Ok(ElyByLoginFlow {
        auth_request_uri: url.to_string(),
    })
}

pub async fn login_finish(
    code: &str,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
) -> crate::Result<Credentials> {
    let token = request_token(&[
        ("grant_type", "authorization_code"),
        ("code", code),
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
        ("redirect_uri", REDIRECT_URI),
    ])
    .await?;

    let info = fetch_account_info(&token.access_token).await?;
    let uuid = Uuid::parse_str(&info.uuid).unwrap_or_else(|_| Uuid::new_v4());

    let credentials = Credentials {
        offline_profile: MinecraftProfile {
            id: uuid,
            name: info.username,
            skins: Vec::new(),
            capes: Vec::new(),
            fetch_time: Some(Instant::now()),
        },
        access_token: token.access_token,
        refresh_token: token.refresh_token.unwrap_or_default(),
        expires: Utc::now() + Duration::seconds(token.expires_in),
        active: true,
        kind: AccountKind::ElyBy,
    };

    credentials.upsert(exec).await?;

    Ok(credentials)
}

pub(super) async fn refresh(
    credentials: &mut Credentials,
    exec: impl sqlx::Executor<'_, Database = sqlx::Sqlite> + Copy,
) -> crate::Result<()> {
    if credentials.expires > Utc::now() + Duration::minutes(5) {
        return Ok(());
    }

    let token = request_token(&[
        ("grant_type", "refresh_token"),
        ("refresh_token", &credentials.refresh_token),
        ("client_id", CLIENT_ID),
        ("client_secret", CLIENT_SECRET),
    ])
    .await?;

    credentials.access_token = token.access_token;
    if let Some(refresh_token) = token.refresh_token {
        credentials.refresh_token = refresh_token;
    }
    credentials.expires = Utc::now() + Duration::seconds(token.expires_in);

    credentials.upsert(exec).await?;

    Ok(())
}

/// Fetches read-only skin/cape texture data for the given Ely.by username. Ely.by does
/// not expose any API to change a skin programmatically, only to read the current one
/// (see docs.ely.by's "Skins system" page) — this is used to populate the read-only
/// skin preview in the existing skins UI.
pub(super) async fn profile(
    username: &str,
    uuid: Uuid,
) -> Result<MinecraftProfile, MinecraftAuthenticationError> {
    let res = INSECURE_REQWEST_CLIENT
        .get(format!("http://skinsystem.ely.by/textures/{username}"))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|source| MinecraftAuthenticationError::Request {
            source,
            step: MinecraftAuthStep::ElyByProfile,
        })?;

    let status = res.status();

    if status == reqwest::StatusCode::NO_CONTENT {
        return Ok(MinecraftProfile {
            id: uuid,
            name: username.to_string(),
            skins: Vec::new(),
            capes: Vec::new(),
            fetch_time: Some(Instant::now()),
        });
    }

    let text = res.text().await.map_err(|source| {
        MinecraftAuthenticationError::Request {
            source,
            step: MinecraftAuthStep::ElyByProfile,
        }
    })?;

    let textures: ElyByTextures =
        serde_json::from_str(&text).map_err(|source| {
            MinecraftAuthenticationError::DeserializeResponse {
                source,
                raw: text,
                step: MinecraftAuthStep::ElyByProfile,
                status_code: status,
            }
        })?;

    Ok(textures.into_profile(username, uuid))
}

#[derive(Deserialize)]
struct ElyByTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    expires_in: i64,
}

#[derive(Deserialize)]
struct ElyByAccountInfo {
    uuid: String,
    username: String,
}

#[derive(Deserialize, Default)]
struct ElyByTextures {
    #[serde(rename = "SKIN")]
    skin: Option<ElyByTexture>,
    #[serde(rename = "CAPE")]
    cape: Option<ElyByTexture>,
}

#[derive(Deserialize)]
struct ElyByTexture {
    url: String,
    #[serde(default)]
    metadata: Option<ElyByTextureMetadata>,
}

#[derive(Deserialize)]
struct ElyByTextureMetadata {
    #[serde(default)]
    model: Option<String>,
}

impl ElyByTextures {
    fn into_profile(self, username: &str, uuid: Uuid) -> MinecraftProfile {
        use super::{MinecraftCape, MinecraftCharacterExpressionState, MinecraftSkin, MinecraftSkinVariant};

        let skins = self
            .skin
            .and_then(|skin| {
                let url = Url::parse(&skin.url).ok()?;
                let variant = match skin
                    .metadata
                    .and_then(|meta| meta.model)
                    .as_deref()
                {
                    Some("slim") => MinecraftSkinVariant::Slim,
                    _ => MinecraftSkinVariant::Classic,
                };

                Some(vec![MinecraftSkin {
                    id: Uuid::new_v4(),
                    state: MinecraftCharacterExpressionState::Active,
                    url: url.into(),
                    texture_key: None,
                    variant,
                    name: None,
                }])
            })
            .unwrap_or_default();

        let capes = self
            .cape
            .and_then(|cape| {
                let url = Url::parse(&cape.url).ok()?;
                Some(vec![MinecraftCape {
                    id: Uuid::new_v4(),
                    state: MinecraftCharacterExpressionState::Active,
                    url: url.into(),
                    name: "Ely.by Cape".into(),
                }])
            })
            .unwrap_or_default();

        MinecraftProfile {
            id: uuid,
            name: username.to_string(),
            skins,
            capes,
            fetch_time: Some(Instant::now()),
        }
    }
}

async fn request_token(
    params: &[(&str, &str)],
) -> crate::Result<ElyByTokenResponse> {
    let res = INSECURE_REQWEST_CLIENT
        .post("https://account.ely.by/api/oauth2/v1/token")
        .header("Accept", "application/json")
        .form(params)
        .send()
        .await?
        .error_for_status()?;

    Ok(res.json().await?)
}

async fn fetch_account_info(
    access_token: &str,
) -> crate::Result<ElyByAccountInfo> {
    let res = INSECURE_REQWEST_CLIENT
        .get("https://account.ely.by/api/account/v1/info")
        .bearer_auth(access_token)
        .send()
        .await?
        .error_for_status()?;

    Ok(res.json().await?)
}
