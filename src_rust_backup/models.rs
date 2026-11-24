use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 新闻类别
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NewsCategory {
    Regulation,      // 监管
    Safety,          // 安全
    Industry,        // 产业
    Technology,      // 技术
    Policy,          // 政策
    Research,        // 研究
}

impl NewsCategory {
    pub fn as_str(&self) -> &str {
        match self {
            NewsCategory::Regulation => "监管",
            NewsCategory::Safety => "安全",
            NewsCategory::Industry => "产业",
            NewsCategory::Technology => "技术",
            NewsCategory::Policy => "政策",
            NewsCategory::Research => "研究",
        }
    }

    pub fn english(&self) -> &str {
        match self {
            NewsCategory::Regulation => "regulation",
            NewsCategory::Safety => "safety",
            NewsCategory::Industry => "industry",
            NewsCategory::Technology => "technology",
            NewsCategory::Policy => "policy",
            NewsCategory::Research => "research",
        }
    }
}

/// 风险等级枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low = 1,
    Medium = 3,
    High = 6,
    Critical = 10,
}

impl RiskLevel {
    pub fn as_str(&self) -> &str {
        match self {
            RiskLevel::Low => "low",
            RiskLevel::Medium => "medium",
            RiskLevel::High => "high",
            RiskLevel::Critical => "critical",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            RiskLevel::Low => "#10b981", // green-500
            RiskLevel::Medium => "#f59e0b", // amber-500
            RiskLevel::High => "#f97316", // orange-500
            RiskLevel::Critical => "#ef4444", // red-500
        }
    }

    pub fn from_int(value: i32) -> Self {
        match value {
            1 => RiskLevel::Low,
            3 => RiskLevel::Medium,
            6 => RiskLevel::High,
            10 => RiskLevel::Critical,
            _ => RiskLevel::Medium,
        }
    }
}

/// 风险趋势
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Trend {
    Increasing,
    Decreasing,
    Stable,
}

impl Trend {
    pub fn as_str(&self) -> &str {
        match self {
            Trend::Increasing => "increasing",
            Trend::Decreasing => "decreasing",
            Trend::Stable => "stable",
        }
    }
}

/// 风险事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskEvent {
    pub id: String,
    pub title_en: String,
    pub title_zh: String,
    pub summary_en: String,
    pub summary_zh: String,
    pub source: String,
    pub source_url: String,
    pub risk_level: RiskLevel,
    pub beneficial: bool,  // 是否有利新闻
    pub timestamp: DateTime<Utc>,
    pub tags: Vec<String>,
    pub category: String,  // 新闻类别
}

impl RiskEvent {
    pub fn new(
        id: String,
        title_en: String,
        title_zh: String,
        summary_en: String,
        summary_zh: String,
        source: String,
        source_url: String,
        risk_level: RiskLevel,
        beneficial: bool,
        timestamp: DateTime<Utc>,
        category: String,
    ) -> Self {
        Self {
            id,
            title_en,
            title_zh,
            summary_en,
            summary_zh,
            source,
            source_url,
            risk_level,
            beneficial,
            timestamp,
            tags: Vec::new(),
            category,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    pub fn hours_ago(&self) -> i64 {
        let now = Utc::now();
        let duration = now - self.timestamp;
        duration.num_hours()
    }

    pub fn is_very_recent(&self) -> bool {
        let now = Utc::now();
        let duration = now - self.timestamp;
        duration.num_seconds() < 3600  // 1小时内
    }
}

/// 风险指数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskIndex {
    pub global_score: f32,
    pub regional_scores: HashMap<String, f32>,
    pub trend: Trend,
    pub last_updated: DateTime<Utc>,
    pub total_events: usize,
    pub critical_events: usize,
}

impl RiskIndex {
    pub fn new(
        global_score: f32,
        regional_scores: HashMap<String, f32>,
        trend: Trend,
        total_events: usize,
        critical_events: usize,
    ) -> Self {
        Self {
            global_score,
            regional_scores,
            trend,
            last_updated: Utc::now(),
            total_events,
            critical_events,
        }
    }

    pub fn update_with_event(&mut self, risk_level: RiskLevel) {
        // 更新全局得分
        let score_multiplier = risk_level as f32 / 10.0;
        self.global_score = (self.global_score * 0.9) + (score_multiplier * 0.1);

        // 更新趋势
        let is_critical = risk_level == RiskLevel::Critical;
        if is_critical && self.trend != Trend::Increasing {
            self.trend = Trend::Increasing;
        }

        self.total_events += 1;
        if is_critical {
            self.critical_events += 1;
        }

        self.last_updated = Utc::now();
    }
}

/// API 响应格式
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// 分页信息
#[derive(Debug, Serialize)]
pub struct Pagination {
    pub page: usize,
    pub per_page: usize,
    pub total: usize,
    pub total_pages: usize,
}

impl Pagination {
    pub fn new(page: usize, per_page: usize, total: usize) -> Self {
        Self {
            page,
            per_page,
            total,
            total_pages: (total as f32 / per_page as f32).ceil() as usize,
        }
    }
}

/// 数据源状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStatus {
    pub name: String,
    pub status: SourceStatusType,
    pub last_checked: DateTime<Utc>,
    pub events_count: usize,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceStatusType {
    Active,
    Error,
    Offline,
}

impl SourceStatus {
    pub fn new(name: String, status: SourceStatusType) -> Self {
        Self {
            name,
            status,
            last_checked: Utc::now(),
            events_count: 0,
            error_message: None,
        }
    }

    pub fn with_events_count(mut self, count: usize) -> Self {
        self.events_count = count;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.status = SourceStatusType::Error;
        self.error_message = Some(error);
        self
    }
}
