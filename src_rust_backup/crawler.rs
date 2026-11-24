use crate::models::{RiskEvent, RiskLevel, NewsCategory};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// 数据源 trait（Send + Sync 使其可跨 tokio task 使用）
pub trait Crawler: Send + Sync {
    fn name(&self) -> &str;
    fn fetch(&mut self) -> Result<Vec<RiskEvent>, CrawlerError>;
}

#[derive(Debug, thiserror::Error)]
pub enum CrawlerError {
    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTML parsing failed: {0}")]
    ParseError(String),

    #[error("No events found")]
    NoEventsFound,
}

impl CrawlerError {
    pub fn to_api_error(&self) -> String {
        format!("Crawler error: {}", self)
    }
}

/// 新闻项
#[derive(Debug, Serialize, Deserialize)]
struct NewsItem {
    title_en: String,
    title_zh: String,
    summary_en: String,
    summary_zh: String,
    source: String,
    url: String,
    risk_level: i32,
    beneficial: bool,
    category: String,
}

/// OpenAI API 爬虫 - 获取 OpenAI 官方公告和新闻
pub struct OpenAICrawler {
    client: Client,
}

impl OpenAICrawler {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap(),
        }
    }

    fn parse_news_item(&self, item: NewsItem) -> Option<RiskEvent> {
        let risk_level = RiskLevel::from_int(item.risk_level);
        let category = NewsCategory::Research;

        Some(RiskEvent::new(
            format!("openai_{}", uuid::Uuid::new_v4()),
            item.title_en,
            item.title_zh,
            item.summary_en,
            item.summary_zh,
            item.source,
            item.url,
            risk_level,
            item.beneficial,
            Utc::now(),
            category.english().to_string(),
        ))
    }
}

impl Crawler for OpenAICrawler {
    fn name(&self) -> &str {
        "OpenAICrawler"
    }

    fn fetch(&mut self) -> Result<Vec<RiskEvent>, CrawlerError> {
        // 模拟获取 OpenAI 官方博客内容
        // 实际应用中可以使用 reqwest 抓取 https://openai.com/blog/
        let news_items = vec![
            NewsItem {
                title_en: "OpenAI Announces New GPT-4.5 Model with Enhanced Capabilities",
                title_zh: "OpenAI 宣布推出 GPT-4.5 模型，性能大幅提升",
                summary_en: "OpenAI 发布了最新的 GPT-4.5 模型，在多个基准测试中表现出色，并改进了指令遵循能力。",
                summary_zh: "OpenAI 发布了最新的 GPT-4.5 模型，在多个基准测试中表现出色，并改进了指令遵循能力。",
                source: "OpenAI Blog",
                url: "https://openai.com/blog/gpt-4.5",
                risk_level: 3,  // Medium
                beneficial: true,  // 有利新闻
                category: "technology".to_string(),
            },
            NewsItem {
                title_en: "OpenAI to Launch New Safety Training Program",
                title_zh: "OpenAI 将推出新的安全训练计划",
                summary_en: "OpenAI 宣布将投资 1 亿美元用于 AI 安全研究，加强模型的可控性和安全性。",
                summary_zh: "OpenAI 宣布将投资 1 亿美元用于 AI 安全研究，加强模型的可控性和安全性。",
                source: "OpenAI Blog",
                url: "https://openai.com/blog/safety-program",
                risk_level: 2,  // Low
                beneficial: true,  // 有利新闻
                category: "safety".to_string(),
            },
            NewsItem {
                title_en: "OpenAI API Rate Limits Adjusted for Enterprise Customers",
                title_zh: "OpenAI 调整企业客户 API 使用配额",
                summary_en: "OpenAI 针对企业客户推出了更灵活的 API 配额方案，提供更高的计算能力和更优的定价。",
                summary_zh: "OpenAI 针对企业客户推出了更灵活的 API 配额方案，提供更高的计算能力和更优的定价。",
                source: "OpenAI Blog",
                url: "https://openai.com/blog/api-limits",
                risk_level: 5,  // Medium
                beneficial: true,  // 有利新闻
                category: "industry".to_string(),
            },
        ];

        Ok(news_items.into_iter().filter_map(|item| self.parse_news_item(item)).collect())
    }
}

/// 新闻网站爬虫 - 获取科技新闻
pub struct NewsCrawler {
    client: Client,
}

impl NewsCrawler {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(15))
                .build()
                .unwrap(),
        }
    }

    fn parse_news_item(&self, item: NewsItem) -> Option<RiskEvent> {
        let risk_level = RiskLevel::from_int(item.risk_level);
        let category = match item.category.as_str() {
            "regulation" => NewsCategory::Regulation,
            "safety" => NewsCategory::Safety,
            "policy" => NewsCategory::Policy,
            _ => NewsCategory::Technology,
        };

        Some(RiskEvent::new(
            format!("news_{}", uuid::Uuid::new_v4()),
            item.title_en,
            item.title_zh,
            item.summary_en,
            item.summary_zh,
            item.source,
            item.url,
            risk_level,
            item.beneficial,
            Utc::now(),
            category.english().to_string(),
        ))
    }
}

impl Crawler for NewsCrawler {
    fn name(&self) -> &str {
        "NewsCrawler"
    }

    fn fetch(&mut self) -> Result<Vec<RiskEvent>, CrawlerError> {
        // 模拟获取科技新闻
        let news_items = vec![
            NewsItem {
                title_en: "EU AI Act Passes Final Vote, Setting Global Standard",
                title_zh: "欧盟 AI 法案通过最终投票，确立全球标准",
                summary_en: "欧盟议会投票通过了 AI Act 法案，建立了全球最严格的 AI 监管框架。",
                summary_zh: "欧盟议会投票通过了 AI Act 法案，建立了全球最严格的 AI 监管框架。",
                source: "TechCrunch",
                url: "https://techcrunch.com/eu-ai-act",
                risk_level: 7,  // High
                beneficial: false,  // 有风险
                category: "regulation".to_string(),
            },
            NewsItem {
                title_en: "Google Gemini Achieves New Performance Records",
                title_zh: "Google Gemini 创造新的性能记录",
                summary_en: "Google 的 Gemini 模型在数学和编程任务上超越了 GPT-4，显示出强大的推理能力。",
                summary_zh: "Google 的 Gemini 模型在数学和编程任务上超越了 GPT-4，显示出强大的推理能力。",
                source: "The Verge",
                url: "https://theverge.com/google-gemini",
                risk_level: 3,  // Medium
                beneficial: true,  // 有利新闻
                category: "technology".to_string(),
            },
            NewsItem {
                title_en: "AI Privacy Concerns Rise as Companies Collect More Data",
                title_zh: "随着公司收集更多数据，AI 隐私担忧上升",
                summary_en: "多家 AI 公司因数据收集问题面临监管审查，用户隐私保护成为关注焦点。",
                summary_zh: "多家 AI 公司因数据收集问题面临监管审查，用户隐私保护成为关注焦点。",
                source: "Wired",
                url: "https://wired.com/ai-privacy",
                risk_level: 5,  // Medium
                beneficial: false,  // 有风险
                category: "safety".to_string(),
            },
            NewsItem {
                title_en: "OpenAI, Microsoft, Google Collaborate on AI Safety Research",
                title_zh: "OpenAI、Microsoft、Google 合作进行 AI 安全研究",
                summary_en: "三大 AI 公司达成协议，共同研究 AI 安全问题，避免重复投入。",
                summary_zh: "三大 AI 公司达成协议，共同研究 AI 安全问题，避免重复投入。",
                source: "AI News",
                url: "https://ainews.com/collaboration",
                risk_level: 2,  // Low
                beneficial: true,  // 有利新闻
                category: "policy".to_string(),
            },
            NewsItem {
                title_en: "New AI Model Released by Anthropic with Enhanced Reasoning",
                title_zh: "Anthropic 发布新模型，推理能力大幅提升",
                summary_en: "Anthropic 推出了 Claude 3 Opus 模型，在复杂推理任务上表现出色。",
                summary_zh: "Anthropic 推出了 Claude 3 Opus 模型，在复杂推理任务上表现出色。",
                source: "Ars Technica",
                url: "https://arstechnica.com/anthropic-claude",
                risk_level: 2,  // Low
                beneficial: true,  // 有利新闻
                category: "technology".to_string(),
            },
            NewsItem {
                title_en: "China Bans AI-generated Political Content",
                title_zh: "中国禁止 AI 生成政治内容",
                summary_en: "中国监管部门发布新规，禁止使用 AI 生成或修改政治内容。",
                summary_zh: "中国监管部门发布新规，禁止使用 AI 生成或修改政治内容。",
                source: "TechCrunch",
                url: "https://techcrunch.com/china-ai-ban",
                risk_level: 8,  // High
                beneficial: false,  // 有风险
                category: "policy".to_string(),
            },
            NewsItem {
                title_en: "Global AI Summit 2024 Results: 50 Countries Commit to Responsible AI",
                title_zh: "全球 AI 峰会 2024 结果：50 国承诺负责任的 AI",
                summary_en: "50 个国家在 AI 峰会上签署协议，承诺在 AI 开发中遵守安全和伦理标准。",
                summary_zh: "50 个国家在 AI 峰会上签署协议，承诺在 AI 开发中遵守安全和伦理标准。",
                source: "AI News",
                url: "https://ainews.com/summit-2024",
                risk_level: 2,  // Low
                beneficial: true,  // 有利新闻
                category: "policy".to_string(),
            },
        ];

        Ok(news_items.into_iter().filter_map(|item| self.parse_news_item(item)).collect())
    }
}

/// Hugging Face 爬虫 - 获取开源模型信息
pub struct HuggingFaceCrawler;

impl HuggingFaceCrawler {
    pub fn new() -> Self {
        Self
    }
}

impl Crawler for HuggingFaceCrawler {
    fn name(&self) -> &str {
        "HuggingFaceCrawler"
    }

    fn fetch(&mut self) -> Result<Vec<RiskEvent>, CrawlerError> {
        Ok(vec![
            RiskEvent::new(
                "hf_1".to_string(),
                "GPT-NeoX-20B: Open Source Alternative to GPT-4",
                "GPT-NeoX-20B: GPT-4 的开源替代方案",
                "Hugging Face releases a new open-source large language model with 20B parameters.",
                "Hugging Face 发布了一个新的 200 亿参数开源大语言模型。",
                "Hugging Face Models".to_string(),
                "https://huggingface.co/models".to_string(),
                RiskLevel::Low,
                true,  // beneficial
                Utc::now(),
                "technology".to_string(),
            ),
            RiskEvent::new(
                "hf_2".to_string(),
                "New Safety Framework for Open Source Models",
                "开源模型的新安全框架",
                "Hugging Face introduces a new safety framework for evaluating open-source AI models.",
                "Hugging Face 为开源 AI 模型引入了新的安全评估框架。",
                "Hugging Face Security".to_string(),
                "https://huggingface.co/security".to_string(),
                RiskLevel::Medium,
                true,  // beneficial
                Utc::now(),
                "safety".to_string(),
            ),
            RiskEvent::new(
                "hf_3".to_string(),
                "Dataset Privacy Tool Released",
                "数据集隐私工具发布",
                "Hugging Face releases a tool for anonymizing datasets, addressing privacy concerns.",
                "Hugging Face 发布了一个数据集匿名化工具，解决隐私担忧。",
                "Hugging Face Tools".to_string(),
                "https://huggingface.co/tools".to_string(),
                RiskLevel::Low,
                true,  // beneficial
                Utc::now(),
                "technology".to_string(),
            ),
        ])
    }
}

/// 爬虫管理器：用 Arc<Mutex<Vec>> 共享，避免 clone 与生命周期问题
pub struct CrawlerManager {
    crawlers: Arc<Mutex<Vec<Box<dyn Crawler>>>>,
    storage: Arc<crate::storage::MemoryStorage>,
}

impl CrawlerManager {
    pub fn new(storage: Arc<crate::storage::MemoryStorage>) -> Self {
        Self {
            crawlers: Arc::new(Mutex::new(Vec::new())),
            storage,
        }
    }

    pub async fn add_crawler(&self, crawler: Box<dyn Crawler>) {
        self.crawlers.lock().await.push(crawler);
    }

    pub async fn crawl_once(&self) {
        let mut crawlers = self.crawlers.lock().await;
        for crawler in crawlers.iter_mut() {
            match crawler.fetch() {
                Ok(events) => {
                    tracing::info!("从 {} 获取到 {} 个事件", crawler.name(), events.len());
                    for event in events {
                        self.storage.add_event(event).await;
                    }
                }
                Err(e) => tracing::error!("爬取失败 {}: {}", crawler.name(), e),
            }
        }
    }

    /// 启动定时调度：克隆 Arc 句柄进 task，不借用 self
    pub fn start_schedule(&self, interval_seconds: u64) {
        let crawlers = self.crawlers.clone();
        let storage = self.storage.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_seconds));
            loop {
                ticker.tick().await;
                let mut list = crawlers.lock().await;
                for crawler in list.iter_mut() {
                    match crawler.fetch() {
                        Ok(events) => {
                            for event in events {
                                storage.add_event(event).await;
                            }
                        }
                        Err(e) => tracing::error!("爬取失败 {}: {}", crawler.name(), e),
                    }
                }
            }
        });
    }
}

pub fn get_available_sources() -> Vec<String> {
    vec![
        "OpenAI Blog".to_string(),
        "TechCrunch".to_string(),
        "The Verge".to_string(),
        "AI News".to_string(),
        "Hugging Face".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_crawler() {
        let mut crawler = OpenAICrawler::new();
        let events = crawler.fetch().unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].title_en.len(), events[0].title_zh.len());
    }

    #[test]
    fn test_news_crawler() {
        let mut crawler = NewsCrawler::new();
        let events = crawler.fetch().unwrap();
        assert!(!events.is_empty());
        assert_eq!(events[0].title_en.len(), events[0].title_zh.len());
    }
}
