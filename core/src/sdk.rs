use axum::{
    extract::Json,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use serde::{Deserialize, Serialize};
use crate::vault_types::{MutateRequest, ReadRequest, ReadResponse, VaultOp};

#[derive(Debug, Clone)]
pub struct GameConfig {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
}

pub struct SdkState {
    pub config: GameConfig,
    pub vault_url: String,
    pub m2m_token: String,
}

pub struct GameServer {
    config: GameConfig,
    router: Router<Arc<SdkState>>,
    static_path: Option<(&'static str, &'static str)>,
}

impl GameServer {
    pub fn new(config: GameConfig) -> Self {
        let router = Router::new();
        Self {
            config,
            router,
            static_path: None,
        }
    }

    pub fn route(mut self, path: &str, method_router: axum::routing::MethodRouter<Arc<SdkState>>) -> Self {
        self.router = self.router.route(path, method_router);
        self
    }

    pub fn static_dir(mut self, nest_path: &'static str, dir_path: &'static str) -> Self {
        self.static_path = Some((nest_path, dir_path));
        self
    }

    pub async fn run(self) {
        let vault_url = std::env::var("VAULT_URL").unwrap_or_else(|_| "http://state-vault:3002".to_string());
        let m2m_token = std::env::var("M2M_TOKEN").unwrap_or_else(|_| format!("secure_{}_m2m_token_abc123", self.config.id));

        let state = Arc::new(SdkState {
            config: self.config.clone(),
            vault_url,
            m2m_token,
        });

        // Add standard health route
        let mut app = self.router
            .route(&format!("/api/games/{}/health", self.config.id), get(health_check));

        // Add static serving if specified
        if let Some((nest_path, dir_path)) = self.static_path {
            app = app.nest_service(nest_path, ServeDir::new(dir_path));
        }

        let app = app
            .with_state(state.clone())
            .layer(CorsLayer::permissive());

        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .unwrap();



        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        println!("🎮 [SDK] Server '{}' listening on {}", self.config.name, addr);

        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();
    }
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "sdk": true }))
}

// Client helper functions for reading/writing balance
pub async fn get_balance(state: &SdkState, user_id: &str) -> i64 {
    let client = reqwest::Client::new();
    let url = format!("{}/api/vault/read", state.vault_url);
    let req = ReadRequest {
        user_id: user_id.to_string(),
        game_id: state.config.id.to_string(),
        key: "balance".to_string(),
    };

    match client.post(&url)
        .header("Authorization", format!("Bearer {}", state.m2m_token))
        .json(&req)
        .send()
        .await
    {
        Ok(resp) => {
            if let Ok(vault_resp) = resp.json::<ReadResponse>().await {
                if let Some(val) = vault_resp.value {
                    if let Some(num) = val.as_i64() {
                        return num;
                    }
                }
            }
        }
        Err(e) => println!("❌ [SDK] Error reading balance for {}: {}", user_id, e),
    }
    1000 // default balance
}

pub async fn mutate_balance(state: &SdkState, user_id: &str, new_balance: i64) -> bool {
    let client = reqwest::Client::new();
    let url = format!("{}/api/vault/mutate", state.vault_url);
    let req = MutateRequest {
        user_id: user_id.to_string(),
        game_id: state.config.id.to_string(),
        key: "balance".to_string(),
        op: VaultOp::Set,
        value: serde_json::json!(new_balance),
    };

    match client.post(&url)
        .header("Authorization", format!("Bearer {}", state.m2m_token))
        .json(&req)
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(e) => {
            println!("❌ [SDK] Error saving balance for {}: {}", user_id, e);
            false
        }
    }
}
