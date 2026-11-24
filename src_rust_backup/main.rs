mod api;
mod crawler;
mod models;
mod storage;
mod websocket;

use axum::{response::Html, routing::get, Router};
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    api::{create_router, AppState},
    crawler::{CrawlerManager, HuggingFaceCrawler, NewsCrawler, OpenAICrawler},
    storage::MemoryStorage,
    websocket::WebSocketHub,
};

fn init_logging() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "ai_risk_monitor=info,tower_http=info".into());

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false);

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    info!("AI 风险监控系统启动中...");

    let storage = Arc::new(MemoryStorage::new());
    let websocket_hub = Arc::new(WebSocketHub::new());

    let app_state = Arc::new(AppState {
        storage: storage.clone(),
        websocket_hub: websocket_hub.clone(),
    });

    let crawler_manager = CrawlerManager::new(storage.clone());
    crawler_manager.add_crawler(Box::new(OpenAICrawler::new()));
    crawler_manager.add_crawler(Box::new(NewsCrawler::new()));
    crawler_manager.add_crawler(Box::new(HuggingFaceCrawler::new()));

    info!("执行初始数据爬取...");
    crawler_manager.crawl_once().await;

    info!("启动爬虫调度器（每 5 分钟）");
    crawler_manager.start_schedule(300);

    // 每 10 秒把最新 5 条事件广播给 WebSocket/SSE 客户端
    let hub = websocket_hub.clone();
    let store = storage.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(10));
        loop {
            ticker.tick().await;
            let events = store.get_latest_events(5).await;
            for event in events {
                hub.broadcast_event(event);
            }
        }
    });

    // 根路由
    let app = Router::new().route("/", get(root));

    // API 子路由（自带 state）
    let app = app.merge(create_router(app_state));

    // CORS
    let app = app.layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    info!("服务器运行在 http://localhost:8080");
    info!("WebSocket 端点: ws://localhost:8080/ws/events");
    info!("SSE 端点: http://localhost:8080/api/events/stream");
    info!("API 端点: http://localhost:8080/api/events");
    info!("前端页面: http://localhost:8080");

    if let Err(e) = axum::serve(listener, app).await {
        error!("服务器错误: {}", e);
    }

    Ok(())
}

async fn root() -> Html<&'static str> {
    Html(include_str!("../templates/index.html"))
}
