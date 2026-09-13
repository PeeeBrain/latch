use crate::vault::storage::VaultStorage;
use crate::vault::workspace::Workspace;
use crate::vault::AliasConfig;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

const SIMPLELOGIN_BASE_URL: &str = "https://app.simplelogin.io";
const DUCKDUCKGO_BASE_URL: &str = "https://quack.duckduckgo.com";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: String,
}

pub trait HttpTransport: Send + Sync {
    fn execute<'a>(
        &'a self,
        request: reqwest::Request,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, String>> + Send + 'a>>;
}

pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl HttpTransport for ReqwestTransport {
    fn execute<'a>(
        &'a self,
        request: reqwest::Request,
    ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let response = self
                .client
                .execute(request)
                .await
                .map_err(|e| format!("Alias request failed: {e}"))?;
            let status = response.status().as_u16();
            let body = response
                .text()
                .await
                .map_err(|e| format!("Failed to read alias response: {e}"))?;
            Ok(HttpResponse { status, body })
        })
    }
}

pub struct AliasClient {
    client: reqwest::Client,
    transport: Box<dyn HttpTransport>,
}

impl AliasClient {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;
        Ok(Self {
            transport: Box::new(ReqwestTransport {
                client: client.clone(),
            }),
            client,
        })
    }

    pub async fn generate(&self, config: &AliasConfig) -> Result<String, String> {
        if config.api_token.trim().is_empty() {
            return Err(format!(
                "Alias provider '{}' has no API token configured",
                config.provider_id
            ));
        }

        let request = build_request(&self.client, config)?;
        let response = self.transport.execute(request).await?;
        parse_response(&config.provider_id, response)
    }

    #[cfg(test)]
    fn with_transport(transport: Box<dyn HttpTransport>) -> Self {
        Self {
            client: reqwest::Client::new(),
            transport,
        }
    }
}

pub fn save_config(
    workspace: &mut Workspace,
    storage: &VaultStorage,
    provider_id: &str,
    api_token: &str,
) -> Result<(), String> {
    workspace.check_session()?;
    workspace.refresh();
    Provider::from_id(provider_id)?;
    if api_token.trim().is_empty() {
        return Err("Alias API token is required".to_string());
    }

    match workspace
        .alias_configs
        .iter_mut()
        .find(|config| config.provider_id == provider_id)
    {
        Some(config) => config.api_token = api_token.to_string(),
        None => workspace.alias_configs.push(AliasConfig {
            provider_id: provider_id.to_string(),
            api_token: api_token.to_string(),
        }),
    }

    crate::vault::entries::persist(workspace, storage)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Provider {
    SimpleLogin,
    DuckDuckGo,
}

impl Provider {
    fn from_id(id: &str) -> Result<Self, String> {
        match id {
            "simplelogin" => Ok(Self::SimpleLogin),
            "duckduckgo" => Ok(Self::DuckDuckGo),
            other => Err(format!("Unsupported alias provider '{other}'")),
        }
    }

    fn response_key(self) -> &'static str {
        match self {
            Self::SimpleLogin => "alias",
            Self::DuckDuckGo => "address",
        }
    }
}

fn build_request(
    client: &reqwest::Client,
    config: &AliasConfig,
) -> Result<reqwest::Request, String> {
    let provider = Provider::from_id(&config.provider_id)?;

    let request = match provider {
        Provider::SimpleLogin => client
            .post(format!("{SIMPLELOGIN_BASE_URL}/api/alias/random/new"))
            .header("authentication", &config.api_token),
        Provider::DuckDuckGo => client
            .post(format!("{DUCKDUCKGO_BASE_URL}/api/email/addresses"))
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", config.api_token),
            ),
    };

    request
        .header(reqwest::header::ACCEPT, "application/json")
        .build()
        .map_err(|e| format!("Failed to build alias request: {e}"))
}

fn parse_response(provider_id: &str, response: HttpResponse) -> Result<String, String> {
    let provider = Provider::from_id(provider_id)?;
    if !(200..300).contains(&response.status) {
        return Err(format!(
            "Alias provider returned status {}",
            response.status
        ));
    }

    let body: serde_json::Value = serde_json::from_str(&response.body)
        .map_err(|_| "Alias provider returned an invalid response".to_string())?;
    let key = provider.response_key();

    body.get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("Alias provider response is missing '{key}'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct MockTransport {
        response: HttpResponse,
        requests: Arc<Mutex<Vec<reqwest::Request>>>,
    }

    impl MockTransport {
        fn new(status: u16, body: &str) -> (Arc<Self>, Arc<Mutex<Vec<reqwest::Request>>>) {
            let requests = Arc::new(Mutex::new(Vec::new()));
            let transport = Arc::new(Self {
                response: HttpResponse {
                    status,
                    body: body.to_string(),
                },
                requests: Arc::clone(&requests),
            });
            (transport, requests)
        }
    }

    impl HttpTransport for Arc<MockTransport> {
        fn execute<'a>(
            &'a self,
            request: reqwest::Request,
        ) -> Pin<Box<dyn Future<Output = Result<HttpResponse, String>> + Send + 'a>> {
            self.requests.lock().unwrap().push(request);
            let response = self.response.clone();
            Box::pin(async move { Ok(response) })
        }
    }

    fn config(provider_id: &str, api_token: &str) -> AliasConfig {
        AliasConfig {
            provider_id: provider_id.to_string(),
            api_token: api_token.to_string(),
        }
    }

    fn client_with_mock(
        status: u16,
        body: &str,
    ) -> (AliasClient, Arc<Mutex<Vec<reqwest::Request>>>) {
        let (transport, requests) = MockTransport::new(status, body);
        (
            AliasClient::with_transport(Box::new(Arc::clone(&transport))),
            requests,
        )
    }

    fn last_request(requests: &Arc<Mutex<Vec<reqwest::Request>>>) -> reqwest::Request {
        let guard = requests.lock().unwrap();
        assert_eq!(guard.len(), 1);
        reqwest::Request::try_clone(guard.last().unwrap()).unwrap()
    }

    #[tokio::test]
    async fn simplelogin_returns_the_alias_from_the_provider_response() {
        let (client, _) = client_with_mock(200, r#"{"alias":"mask@simplelogin.io"}"#);

        let generated = client
            .generate(&config("simplelogin", "sl-token"))
            .await
            .unwrap();

        assert_eq!(generated, "mask@simplelogin.io");
    }

    #[tokio::test]
    async fn simplelogin_sends_the_token_in_the_authentication_header() {
        let (client, requests) = client_with_mock(200, r#"{"alias":"mask@simplelogin.io"}"#);

        client
            .generate(&config("simplelogin", "sl-token"))
            .await
            .unwrap();

        let request = last_request(&requests);
        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(
            request.url().as_str(),
            "https://app.simplelogin.io/api/alias/random/new"
        );
        assert_eq!(request.headers().get("authentication").unwrap(), "sl-token");
        assert!(request
            .headers()
            .get(reqwest::header::AUTHORIZATION)
            .is_none());
    }

    #[tokio::test]
    async fn duckduckgo_sends_the_token_as_a_bearer_token() {
        let (client, requests) = client_with_mock(200, r#"{"address":"mask@duck.com"}"#);

        let generated = client
            .generate(&config("duckduckgo", "ddg-token"))
            .await
            .unwrap();

        assert_eq!(generated, "mask@duck.com");
        let request = last_request(&requests);
        assert_eq!(request.method(), reqwest::Method::POST);
        assert_eq!(
            request.url().as_str(),
            "https://quack.duckduckgo.com/api/email/addresses"
        );
        assert_eq!(
            request
                .headers()
                .get(reqwest::header::AUTHORIZATION)
                .unwrap(),
            "Bearer ddg-token"
        );
    }

    #[tokio::test]
    async fn provider_failures_surface_the_status_code() {
        let (client, _) = client_with_mock(401, r#"{"error":"invalid token"}"#);

        let error = client
            .generate(&config("simplelogin", "bad-token"))
            .await
            .unwrap_err();

        assert_eq!(error, "Alias provider returned status 401");
    }

    #[tokio::test]
    async fn malformed_provider_responses_are_rejected() {
        let (client, _) = client_with_mock(200, r#"{"unexpected":"shape"}"#);

        let error = client
            .generate(&config("duckduckgo", "ddg-token"))
            .await
            .unwrap_err();

        assert_eq!(error, "Alias provider response is missing 'address'");
    }

    #[tokio::test]
    async fn unsupported_providers_are_rejected() {
        let (client, requests) = client_with_mock(200, r#"{"alias":"mask@example.com"}"#);

        let error = client
            .generate(&config("cloudflare", "token"))
            .await
            .unwrap_err();

        assert_eq!(error, "Unsupported alias provider 'cloudflare'");
        assert!(requests.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn missing_tokens_are_rejected_before_any_request() {
        let (client, requests) = client_with_mock(200, r#"{"alias":"mask@example.com"}"#);

        let error = client
            .generate(&config("simplelogin", "   "))
            .await
            .unwrap_err();

        assert_eq!(
            error,
            "Alias provider 'simplelogin' has no API token configured"
        );
        assert!(requests.lock().unwrap().is_empty());
    }

    fn storage_with_existing_vault() -> (crate::vault::storage::VaultStorage, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let storage = crate::vault::storage::VaultStorage {
            path: dir.path().join("vault.enc"),
        };
        storage
            .write(&crate::vault::EncryptedVault {
                version: "1".to_string(),
                kdf: "argon2".to_string(),
                salt: "00".to_string(),
                data: crate::crypto::aead::EncryptedData {
                    nonce: "00".to_string(),
                    ciphertext: "00".to_string(),
                },
            })
            .unwrap();
        (storage, dir)
    }

    fn unlocked_workspace() -> crate::vault::workspace::Workspace {
        let mut workspace = crate::vault::workspace::Workspace::new();
        workspace.start([9u8; 32]);
        workspace
    }

    #[test]
    fn saved_configs_are_written_to_the_encrypted_vault() {
        let (storage, _dir) = storage_with_existing_vault();
        let mut workspace = unlocked_workspace();
        let key = [9u8; 32];

        save_config(&mut workspace, &storage, "simplelogin", "sl-token").unwrap();

        let vault = storage.read().unwrap();
        let decrypted = crate::crypto::aead::decrypt(&key, &vault.data).unwrap();
        let data: crate::vault::VaultData = serde_json::from_str(&decrypted).unwrap();
        assert_eq!(data.alias_configs.len(), 1);
        assert_eq!(data.alias_configs[0].provider_id, "simplelogin");
        assert_eq!(data.alias_configs[0].api_token, "sl-token");
    }

    #[test]
    fn saving_the_same_provider_replaces_its_token() {
        let (storage, _dir) = storage_with_existing_vault();
        let mut workspace = unlocked_workspace();

        save_config(&mut workspace, &storage, "duckduckgo", "old-token").unwrap();
        save_config(&mut workspace, &storage, "duckduckgo", "new-token").unwrap();

        assert_eq!(workspace.alias_configs.len(), 1);
        assert_eq!(workspace.alias_configs[0].api_token, "new-token");
    }

    #[test]
    fn unsupported_providers_are_rejected_when_saving() {
        let (storage, _dir) = storage_with_existing_vault();
        let mut workspace = unlocked_workspace();

        let error = save_config(&mut workspace, &storage, "cloudflare", "token").unwrap_err();

        assert_eq!(error, "Unsupported alias provider 'cloudflare'");
        assert!(workspace.alias_configs.is_empty());
    }

    #[test]
    fn empty_tokens_are_rejected_when_saving() {
        let (storage, _dir) = storage_with_existing_vault();
        let mut workspace = unlocked_workspace();

        let error = save_config(&mut workspace, &storage, "simplelogin", "   ").unwrap_err();

        assert_eq!(error, "Alias API token is required");
        assert!(workspace.alias_configs.is_empty());
    }

    #[test]
    fn saving_a_config_rejects_locked_vaults() {
        let (storage, _dir) = storage_with_existing_vault();
        let mut workspace = crate::vault::workspace::Workspace::new();

        let error = save_config(&mut workspace, &storage, "simplelogin", "sl-token").unwrap_err();

        assert_eq!(error, "Vault is locked");
    }
}
