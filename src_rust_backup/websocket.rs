use axum::{
    extract::ws::{Message, WebSocket},
    response::sse::{Event as SseEvent, Sse},
};
use futures::{stream, StreamExt};
use serde::Serialize;
use std::{
    convert::Infallible,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::broadcast;
use tracing::{error, info};

use crate::models::RiskEvent;

/// WebSocket 广播中心
pub struct WebSocketHub {
    sender: broadcast::Sender<RiskEvent>,
    subscriber_count: Arc<AtomicUsize>,
}

impl WebSocketHub {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self {
            sender,
            subscriber_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 处理一个 WebSocket 连接
    pub async fn handle_connection(socket: WebSocket, hub: Arc<Self>) {
        let count = hub.subscriber_count.fetch_add(1, Ordering::Relaxed) + 1;
        info!("WebSocket 连接建立，当前订阅者: {}", count);

        let (mut sender, mut receiver) = socket.split();
        let mut rx = hub.sender.subscribe();

        // 循环：把广播事件写入 socket，同时监听客户端关闭
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(event) => {
                            let text = serde_json::to_string(&event).unwrap_or_default();
                            if sender.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            error!("WebSocket 客户端落后 {} 条事件", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                msg = receiver.next() => {
                    match msg {
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Err(e)) => {
                            error!("WebSocket 接收错误: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        let count = hub.subscriber_count.fetch_sub(1, Ordering::Relaxed) - 1;
        info!("WebSocket 连接断开，剩余订阅者: {}", count);
    }

    /// 向所有订阅者广播事件
    pub fn broadcast_event(&self, event: RiskEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscriber_count(&self) -> usize {
        self.subscriber_count.load(Ordering::Relaxed)
    }

    /// 订阅事件流（供 SSE 使用）
    fn subscribe(&self) -> broadcast::Receiver<RiskEvent> {
        self.sender.subscribe()
    }
}

impl Default for WebSocketHub {
    fn default() -> Self {
        Self::new()
    }
}

/// SSE 处理器：把广播事件转成 Server-Sent Events
pub async fn handle_sse(
    axum::extract::State(hub): axum::extract::State<Arc<WebSocketHub>>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    let rx = hub.subscribe();
    let stream = stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_default();
                    let e = SseEvent::default().event("risk_event").data(json);
                    return Some((Ok(e), rx));
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => return None,
            }
        }
    });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

/// 事件转换器
#[derive(Debug, Serialize)]
pub struct StreamEvent {
    pub id: String,
    pub title: String,
    pub summary: String,
    pub source: String,
    pub risk_level: String,
    pub timestamp: String,
    pub url: String,
}

impl From<RiskEvent> for StreamEvent {
    fn from(event: RiskEvent) -> Self {
        Self {
            id: event.id,
            title: event.title,
            summary: event.summary,
            source: event.source,
            risk_level: event.risk_level.as_str().to_string(),
            timestamp: event.timestamp.to_rfc3339(),
            url: event.url,
        }
    }
}
