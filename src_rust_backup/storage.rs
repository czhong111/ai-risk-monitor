use crate::models::{RiskEvent, RiskIndex, Trend};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 内存存储实现
pub struct MemoryStorage {
    events: Arc<RwLock<Vec<RiskEvent>>>,
    index: Arc<RwLock<RiskIndex>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        let events = Arc::new(RwLock::new(Vec::new()));
        let index = Arc::new(RwLock::new(RiskIndex::new(
            5.0,
            HashMap::new(),
            Trend::Stable,
            0,
            0,
        )));

        Self { events, index }
    }

    /// 添加风险事件
    pub async fn add_event(&self, event: RiskEvent) {
        let mut events = self.events.write().await;
        if events.len() >= 1000 {
            events.remove(0);
        }
        events.push(event.clone());

        // 清理 30 天前的事件
        let cutoff = Utc::now() - chrono::Duration::days(30);
        events.retain(|e| e.timestamp > cutoff);

        let mut index = self.index.write().await;
        index.update_with_event(event.risk_level);
    }

    /// 获取所有事件
    pub async fn get_events(&self) -> Vec<RiskEvent> {
        self.events.read().await.clone()
    }

    /// 按风险等级筛选
    pub async fn get_events_by_level(&self, level: crate::models::RiskLevel) -> Vec<RiskEvent> {
        self.events
            .read()
            .await
            .iter()
            .filter(|e| e.risk_level == level)
            .cloned()
            .collect()
    }

    /// 按来源筛选
    pub async fn get_events_by_source(&self, source: &str) -> Vec<RiskEvent> {
        self.events
            .read()
            .await
            .iter()
            .filter(|e| e.source == source)
            .cloned()
            .collect()
    }

    /// 获取风险指数
    pub async fn get_risk_index(&self) -> RiskIndex {
        self.index.read().await.clone()
    }

    /// 获取最新事件
    pub async fn get_latest_events(&self, limit: usize) -> Vec<RiskEvent> {
        self.events
            .read()
            .await
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }

    /// 清空数据
    pub async fn clear(&self) {
        let mut events = self.events.write().await;
        let mut index = self.index.write().await;
        events.clear();
        *index = RiskIndex::new(5.0, HashMap::new(), Trend::Stable, 0, 0);
    }

    /// 事件总数
    pub async fn get_total_events(&self) -> usize {
        self.events.read().await.len()
    }

    /// 订阅者数量（供 API 展示）
    pub async fn subscriber_count(&self) -> usize {
        0
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_event() {
        let storage = MemoryStorage::new();
        let event = RiskEvent::new(
            "1".to_string(),
            "Test Event".to_string(),
            "Test Summary".to_string(),
            "Test Source".to_string(),
            crate::models::RiskLevel::High,
            Utc::now(),
            "http://example.com".to_string(),
        );

        storage.add_event(event).await;
        let events = storage.get_events().await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "1");
    }

    #[tokio::test]
    async fn test_risk_index_update() {
        let storage = MemoryStorage::new();
        let event = RiskEvent::new(
            "1".to_string(),
            "Critical Event".to_string(),
            "Summary".to_string(),
            "Source".to_string(),
            crate::models::RiskLevel::Critical,
            Utc::now(),
            "http://example.com".to_string(),
        );

        storage.add_event(event).await;
        let index = storage.get_risk_index().await;
        // 5.0 * 0.9 + (10/10 * 0.1) = 4.6
        assert!((index.global_score - 4.6).abs() < 1e-6);
        assert_eq!(index.critical_events, 1);
    }
}
