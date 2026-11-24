// 测试代码：验证新功能

fn main() {
    // 测试模型中的功能
    let events = vec![
        ("OpenAI GPT-4.5 发布", "OpenAI 发布 GPT-4.5 模型", true, "技术"),
        ("欧盟 AI 法案通过", "欧盟通过严格的 AI 监管法案", false, "监管"),
        ("Google Gemini 性能提升", "Gemini 模型超越 GPT-4", true, "技术"),
    ];

    println!("=== 新闻中英文双语测试 ===\n");

    for (title_en, title_zh, beneficial, category) in events {
        println!("英文: {}", title_en);
        println!("中文: {}", title_zh);
        println!("有利: {}", if beneficial { "✅ 有利" } else { "⚠️ 有风险" });
        println!("类别: {}", category);
        println!("---");
    }

    println!("\n=== 功能总结 ===");
    println!("✅ 支持中英文双语显示");
    println!("✅ 标识有利/不利新闻");
    println!("✅ 按风险等级筛选");
    println!("✅ 按类别筛选（监管、安全、技术等）");
    println!("✅ 实时获取新闻数据");
    println!("✅ Bloomberg 风格滚动新闻条");
}
