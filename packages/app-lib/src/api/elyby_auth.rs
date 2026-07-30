//! Authentication flow interface for Ely.by accounts

use crate::State;
use crate::state::Credentials;
use crate::state::elyby_auth::ElyByLoginFlow;
pub use crate::state::elyby_auth::REDIRECT_PORT;

#[tracing::instrument]
pub fn begin_login() -> crate::Result<ElyByLoginFlow> {
    crate::state::elyby_auth::begin_login()
}

#[tracing::instrument]
pub async fn finish_login(code: &str) -> crate::Result<Credentials> {
    let state = State::get().await?;

    crate::state::elyby_auth::login_finish(code, &state.pool).await
}
