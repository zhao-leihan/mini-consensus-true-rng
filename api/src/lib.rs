use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use consensus::{ConsensusState, VotePhase};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use trng::Trng;
use tower_http::cors::CorsLayer;

#[derive(Clone)]
pub struct AppState {
    pub consensus: ConsensusState,
    pub trng: Trng,
}

#[derive(Debug, Deserialize)]
pub struct ProposeRequest {
    pub payload: String,
}

#[derive(Debug, Deserialize)]
pub struct VoteRequest {
    pub proposal_id: String,
    pub validator_id: usize,
    pub phase: String,
}

#[derive(Debug, Deserialize)]
pub struct RngQuery {
    pub len: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct ProposeResponse {
    pub proposal_id: String,
}

#[derive(Debug, Serialize)]
pub struct VoteResponse {
    pub success: bool,
    pub finalized: bool,
}

#[derive(Debug, Serialize)]
pub struct FinalizedResponse {
    pub finalized_block: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RngResponse {
    pub random_bytes: String, // hex encoded
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub healthy: bool,
    pub metrics: HashMap<String, f64>,
}

pub async fn start_server(port: u16) {
    let validators = vec![0, 1, 2, 3];
    let app_state = AppState {
        consensus: ConsensusState::new(validators),
        trng: Trng::new(),
    };

    let app = Router::new()
        .route("/finalized", get(get_finalized))
        .route("/propose", post(propose))
        .route("/vote", post(vote))
        .route("/rng", get(get_rng))
        .route("/health", get(health_check))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .unwrap();
    
    println!("Server running on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await.unwrap();
}

async fn get_finalized(
    State(state): State<AppState>,
) -> Json<FinalizedResponse> {
    let finalized_block = state.consensus.finalize();
    
    Json(FinalizedResponse {
        finalized_block,
    })
}

async fn propose(
    State(state): State<AppState>,
    Json(payload): Json<ProposeRequest>,
) -> Result<Json<ProposeResponse>, StatusCode> {
    let proposal_id = state.consensus.propose(payload.payload.into_bytes());
    
    Ok(Json(ProposeResponse {
        proposal_id,
    }))
}

async fn vote(
    State(state): State<AppState>,
    Json(vote_req): Json<VoteRequest>,
) -> Json<VoteResponse> {
    let phase = match vote_req.phase.as_str() {
        "precommit" => VotePhase::Precommit,
        "commit" => VotePhase::Commit,
        _ => {
            return Json(VoteResponse {
                success: false,
                finalized: false,
            });
        }
    };

    let success = state.consensus.vote(vote_req.proposal_id, vote_req.validator_id, phase);
    let finalized = state.consensus.finalize().is_some();
    
    Json(VoteResponse {
        success,
        finalized,
    })
}

async fn get_rng(
    State(state): State<AppState>,
    Query(params): Query<RngQuery>,
) -> Json<RngResponse> {
    let len = params.len.unwrap_or(32);
    let random_bytes = state.trng.rand_bytes(len);
    
    Json(RngResponse {
        random_bytes: hex::encode(random_bytes),
    })
}

async fn health_check(
    State(state): State<AppState>,
) -> Json<HealthResponse> {
    let health = state.trng.health_check(8192);
    
    let mut metrics = HashMap::new();
    metrics.insert("monobit_deviation".to_string(), health.monobit_deviation);
    metrics.insert("runs_deviation".to_string(), health.runs_deviation);
    metrics.insert("shannon_entropy".to_string(), health.shannon_entropy);
    
    Json(HealthResponse {
        healthy: health.is_healthy(),
        metrics,
    })
}