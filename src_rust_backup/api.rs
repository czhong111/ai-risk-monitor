use axum::{
    extract::{Query, State, WebSocketUpgrade},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    crawler::get_available_sources,
    models::{
        ApiResponse, NewsCategory, Pagination, RiskEvent, RiskIndex, SourceStatus,
        SourceStatusType,
    },
    storage::MemoryStorage,
    websocket::{handle_sse, WebSocketHub},
};

/// API 状态
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<MemoryStorage>,
    pub websocket_hub: Arc<WebSocketHub>,
}

/// 查询参数
#[derive(Debug, Deserialize, Default)]
pub struct EventsQuery {
    pub page: usize,
    pub per_page: usize,
    pub beneficial_only: Option<bool>,  // 只显示有利新闻
    pub category: Option<String>,      // 按类别筛选
}

impl EventsQuery {
    pub fn page(&self) -> usize {
        self.page.max(1)
    }

    pub fn per_page(&self) -> usize {
        self.per_page.clamp(1, 100)
    }
}

/// GET /api/events —— 风险事件列表（分页）
pub async fn get_events(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EventsQuery>,
) -> Json<ApiResponse<Vec<RiskEvent>>> {
    let events = state.storage.get_events().await;

    // 筛选
    let mut filtered = events;
    if let Some(beneficial_only) = params.beneficial_only {
        filtered.retain(|e| e.beneficial == beneficial_only);
    }
    if let Some(category) = params.category {
        filtered.retain(|e| e.category == category);
    }

    // 分页
    let page = params.page();
    let per_page = params.per_page();
    let start = (page - 1) * per_page;
    let paginated: Vec<RiskEvent> = filtered
        .into_iter()
        .skip(start)
        .take(per_page)
        .collect();

    let pagination = Pagination::new(page, per_page, filtered.len());

    Json(ApiResponse {
        success: true,
        data: Some(paginated),
        error: None,
    })
}

/// GET /api/risk-index
pub async fn get_risk_index(State(state): State<Arc<AppState>>) -> Json<ApiResponse<RiskIndex>> {
    let index = state.storage.get_risk_index().await;
    Json(ApiResponse {
        success: true,
        data: Some(index),
        error: None,
    })
}

/// GET /api/sources
pub async fn get_sources() -> Json<ApiResponse<Vec<String>>> {
    Json(ApiResponse {
        success: true,
        data: Some(get_available_sources()),
        error: None,
    })
}

/// GET /api/sources/status
pub async fn get_source_status() -> Json<ApiResponse<Vec<SourceStatus>>> {
    let statuses = vec![
        SourceStatus::new("OpenAI Blog".to_string(), SourceStatusType::Active),
        SourceStatus::new("TechCrunch".to_string(), SourceStatusType::Active),
        SourceStatus::new("The Verge".to_string(), SourceStatusType::Active),
        SourceStatus::new("AI News".to_string(), SourceStatusType::Active),
        SourceStatus::new("Hugging Face".to_string(), SourceStatusType::Active),
    ];
    Json(ApiResponse {
        success: true,
        data: Some(statuses),
        error: None,
    })
}

/// GET /api/events/latest —— 获取最新事件
pub async fn get_latest_events(
    State(state): State<Arc<AppState>>,
    Query(params): Query<EventsQuery>,
) -> Json<ApiResponse<Vec<RiskEvent>>> {
    let events = state.storage.get_latest_events(params.per_page()).await;

    // 筛选
    let mut filtered = events;
    if let Some(beneficial_only) = params.beneficial_only {
        filtered.retain(|e| e.beneficial == beneficial_only);
    }
    if let Some(category) = params.category {
        filtered.retain(|e| e.category == category);
    }

    Json(ApiResponse {
        success: true,
        data: Some(filtered),
        error: None,
    })
}

/// GET /api/events/beneficial —— 只获取有利新闻
pub async fn get_beneficial_events(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<Vec<RiskEvent>>> {
    let all_events = state.storage.get_events().await;
    let beneficial = all_events.into_iter().filter(|e| e.beneficial).collect();

    Json(ApiResponse {
        success: true,
        data: Some(beneficial),
        error: None,
    })
}

/// GET /api/events/by-category —— 按类别获取
pub async fn get_events_by_category(
    State(state): State<Arc<AppState>>,
    Query(category): Query<String>,
) -> Json<ApiResponse<Vec<RiskEvent>>> {
    let all_events = state.storage.get_events().await;
    let filtered = all_events
        .into_iter()
        .filter(|e| e.category == category)
        .collect();

    Json(ApiResponse {
        success: true,
        data: Some(filtered),
        error: None,
    })
}

/// GET /api/categories —— 获取所有类别
pub async fn get_categories() -> Json<ApiResponse<Vec<String>>> {
    let categories = vec![
        "regulation".to_string(),
        "safety".to_string(),
        "industry".to_string(),
        "technology".to_string(),
        "policy".to_string(),
        "research".to_string(),
    ];
    Json(ApiResponse {
        success: true,
        data: Some(categories),
        error: None,
    })
}

/// GET /ws/events —— WebSocket 升级
pub async fn ws_events(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let hub = state.websocket_hub.clone();
    ws.on_upgrade(move |socket| WebSocketHub::handle_connection(socket, hub))
}

/// GET /api/events/stream —— SSE 流
pub async fn sse_events(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let hub = state.websocket_hub.clone();
    handle_sse(state.websocket_hub.clone()).await
}

/// POST /api/events/refresh —— 手动触发刷新
pub async fn refresh_events(
    State(state): State<Arc<AppState>>,
) -> Json<ApiResponse<String>> {
    // 在实际实现中，这里会调用爬虫
    // 目前返回模拟响应
    Json(ApiResponse {
        success: true,
        data: Some("爬虫已触发（模拟）".to_string()),
        error: None,
    })
}

/// API 路由
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // RESTful API
        .route("/api/events", get(get_events))
        .route("/api/events/latest", get(get_latest_events))
        .route("/api/events/beneficial", get(get_beneficial_events))
        .route("/api/events/by-category", get(get_events_by_category))
        .route("/api/risk-index", get(get_risk_index))
        .route("/api/sources", get(get_sources))
        .route("/api/sources/status", get(get_source_status))
        .route("/api/categories", get(get_categories))
        .route("/api/events/refresh", post(refresh_events))
        // WebSocket
        .route("/ws/events", get(ws_events))
        // SSE
        .route("/api/events/stream", get(sse_events))
        .layer(axum::Extension(state))
}
