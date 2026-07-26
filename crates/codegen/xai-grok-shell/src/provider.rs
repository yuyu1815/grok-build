//! Grok-only provider boundary.
//!
//! This module intentionally owns no credentials, session actors, or model
//! catalog. It provides the small amount of provider identity and lifecycle
//! orchestration that the shell and pager need while reusing the existing auth
//! and model managers.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};

use crate::agent::models::ModelsManager;
use crate::auth::{
    self, AuthManager, AuthUrlInfo, GrokAuth, LoginTransportOverride, StderrCallback,
};

/// Internal canonical provider identifier.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
pub enum ProviderId {
    #[serde(rename = "xai")]
    Xai,
    /// Custom or otherwise unclassified models must not inherit xAI secrets.
    #[default]
    #[serde(rename = "unknown")]
    Unknown,
}

impl ProviderId {
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Xai => "Grok",
            Self::Unknown => "Custom",
        }
    }

    /// Normalize the explicitly supported user-facing provider name.
    pub fn from_ui_name(name: &str) -> Option<Self> {
        name.eq_ignore_ascii_case("grok").then_some(Self::Xai)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderDescriptor {
    pub id: ProviderId,
    pub display_name: &'static str,
}

/// Fixed registry for the providers implemented by this binary.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProviderRegistry;

impl ProviderRegistry {
    pub const fn new() -> Self {
        Self
    }

    pub const fn providers(self) -> &'static [ProviderDescriptor] {
        &[ProviderDescriptor {
            id: ProviderId::Xai,
            display_name: "Grok",
        }]
    }

    pub fn resolve_ui_name(self, name: &str) -> Option<ProviderId> {
        ProviderId::from_ui_name(name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogFetchState {
    NotAttempted,
    Empty,
    Failed,
    StaleCache,
    Ready,
}

/// Safe, provider-neutral model state for status and command output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProviderModelState {
    pub authenticated: bool,
    pub selectable_real_models: usize,
    pub real_catalog_fetched: bool,
    pub allowlist_excludes_all: bool,
    pub fetch_state: CatalogFetchState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderChatGate {
    Ready,
    LoginRequired,
    NoAvailableModels,
    ModelFetchFailed,
    AllowlistExcludesAll,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthStatusSnapshot {
    pub provider: ProviderId,
    pub authenticated: bool,
    pub auth_method: Option<String>,
    pub user_id: Option<String>,
    pub email: Option<String>,
    pub team_id: Option<String>,
}

/// Shared Grok auth/model lifecycle facade.
#[derive(Clone)]
pub struct GrokProvider {
    auth_manager: Arc<AuthManager>,
    models_manager: ModelsManager,
}

impl GrokProvider {
    pub fn new(auth_manager: Arc<AuthManager>, models_manager: ModelsManager) -> Self {
        Self {
            auth_manager,
            models_manager,
        }
    }

    pub const fn id(&self) -> ProviderId {
        ProviderId::Xai
    }

    pub fn auth_status(&self) -> AuthStatusSnapshot {
        let auth = self.auth_manager.current();
        let has_api_key_env = crate::agent::auth_method::has_xai_api_key_env();
        AuthStatusSnapshot {
            provider: self.id(),
            authenticated: (auth.is_some() && !self.auth_manager.is_expired()) || has_api_key_env,
            auth_method: auth
                .as_ref()
                .map(|a| format!("{:?}", a.auth_mode))
                .or_else(|| has_api_key_env.then_some("ApiKey".to_owned())),
            user_id: auth.as_ref().map(|a| a.user_id.clone()),
            email: auth.as_ref().and_then(|a| a.email.clone()),
            team_id: auth.as_ref().and_then(|a| a.team_id.clone()),
        }
    }

    pub fn model_state(&self) -> ProviderModelState {
        self.models_manager
            .provider_model_state(self.auth_status().authenticated)
    }

    pub fn chat_gate(&self) -> ProviderChatGate {
        let state = self.model_state();
        let has_model_credentials = self.models_manager.has_current_model_credentials();
        let has_global_api_key = crate::agent::auth_method::has_xai_api_key_env();
        let has_deployment_key = self.models_manager.endpoints().deployment_key.is_some();
        if !state.authenticated
            && !has_global_api_key
            && !has_model_credentials
            && !has_deployment_key
        {
            return ProviderChatGate::LoginRequired;
        }
        if state.allowlist_excludes_all {
            return ProviderChatGate::AllowlistExcludesAll;
        }
        if has_model_credentials && state.selectable_real_models == 0 {
            return ProviderChatGate::Ready;
        }
        if state.selectable_real_models == 0 {
            return match state.fetch_state {
                CatalogFetchState::Failed | CatalogFetchState::StaleCache => {
                    ProviderChatGate::ModelFetchFailed
                }
                _ => ProviderChatGate::NoAvailableModels,
            };
        }
        ProviderChatGate::Ready
    }

    pub async fn login_with_channels(
        &self,
        channels: auth::AuthChannels,
        reauth: bool,
        interactive: bool,
        force_interactive: bool,
        login_override: LoginTransportOverride,
    ) -> anyhow::Result<GrokAuth> {
        let (auth, _) = if interactive {
            auth::run_auth_flow_with_stderr_bridge(
                &self.auth_manager,
                self.auth_manager.grok_com_config(),
                channels,
                reauth,
                force_interactive,
                login_override,
            )
            .await?
        } else {
            auth::run_auth_flow(
                &self.auth_manager,
                self.auth_manager.grok_com_config(),
                reauth,
                None,
                None,
                None,
                login_override,
            )
            .await?
        };
        self.on_auth_changed().await;
        Ok(auth)
    }

    pub async fn login(
        &self,
        on_stderr: Option<StderrCallback>,
        url_tx: Option<Rc<RefCell<Option<oneshot::Sender<AuthUrlInfo>>>>>,
        code_rx: Option<mpsc::Receiver<String>>,
        login_override: LoginTransportOverride,
    ) -> anyhow::Result<GrokAuth> {
        let (auth, _) = auth::run_auth_flow_interactive(
            &self.auth_manager,
            &self.auth_manager.grok_com_config(),
            on_stderr,
            url_tx,
            code_rx,
            login_override,
        )
        .await?;
        self.on_auth_changed().await;
        Ok(auth)
    }

    pub fn logout(&self) -> std::io::Result<auth::LogoutResult> {
        self.logout_scope(None)
    }

    pub fn logout_scope(&self, scope: Option<&str>) -> std::io::Result<auth::LogoutResult> {
        auth::perform_logout(&self.auth_manager, scope)
    }

    pub async fn logout_and_reload(
        &self,
        scope: Option<&str>,
    ) -> std::io::Result<auth::LogoutResult> {
        let result = self.logout_scope(scope)?;
        self.on_auth_changed().await;
        Ok(result)
    }

    pub async fn on_auth_changed(&self) {
        self.models_manager.on_auth_changed().await;
    }

    pub fn auth_manager(&self) -> &Arc<AuthManager> {
        &self.auth_manager
    }

    pub fn models_manager(&self) -> &ModelsManager {
        &self.models_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_only_grok() {
        assert_eq!(ProviderId::from_ui_name("grok"), Some(ProviderId::Xai));
        assert_eq!(ProviderId::from_ui_name("GROK"), Some(ProviderId::Xai));
        assert_eq!(ProviderId::from_ui_name("xai"), None);
        assert_eq!(ProviderId::from_ui_name("codex"), None);
    }

    #[test]
    fn registry_is_grok_only_and_keeps_internal_id_separate() {
        let providers = ProviderRegistry::new().providers();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, ProviderId::Xai);
        assert_eq!(providers[0].display_name, "Grok");
        assert_eq!(ProviderId::Xai.display_name(), "Grok");
    }

    #[test]
    fn rejects_internal_or_unknown_provider_names() {
        assert_eq!(ProviderRegistry::new().resolve_ui_name("xai"), None);
        assert_eq!(ProviderRegistry::new().resolve_ui_name("codex"), None);
    }
}
