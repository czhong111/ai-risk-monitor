# AI 风险监控系统

一个实时监控全球 AI 风险的 Web Dashboard，提供要闻展示和风险指数功能。

**当前版本**：纯 Python 标准库实现，无需编译，零依赖。

## 快速开始

### 1. 启动服务

```bash
cd C:\Users\Administrator\Documents\ai-risk-monitor
python web_server.py
```

然后在浏览器打开 **http://localhost:8080**

### 2. 守护进程（推荐）

```bash
python watchdog.py --loop
```

守护进程会：
- 检查 Web 服务健康状态
- 服务挂了自动重启
- 扫描 Claude Code 会话日志，检测错误（GLM 限流、权限拦截等）

---

## 项目结构

```
ai-risk-monitor/
├── web_server.py          # Web 服务（345 行）
├── watchdog.py            # 守护进程
├── status.json            # 服务状态（自动生成）
├── templates/             # 静态页面
│   ├── index.html         # 主页面
│   ├── styles.css         # 样式
│   └── script.js          # 前端脚本
├── src_rust_backup/       # Rust 版本备份（未编译）
├── .bak_original/         # 原始备份
└── README.md              # 本文档
```

---

## 功能特性

### 核心功能

- **RSS 新闻源监控**：并发拉取 9 个国际新闻源
  - Hacker News
  - Hacker News · AI
  - TechCrunch
  - The Verge
  - Wired
  - Ars Technica
  - OpenAI News
  - arXiv · cs.AI
  - NPR

- **AI 风险评分**：基于关键词库的四维评分
  - 估值泡沫（权重 3.0）
  - 监管政策（权重 2.2）
  - 安全风险（权重 2.6）
  - 产业冲击（权重 1.8）

- **风险指数计算**：0-10 分全球风险评分
  - 时间衰减机制（最近 6 小时的高风险新闻加权）
  - 趋势分析（上升/下降/稳定）

- **实时缓存**：3 分钟 TTL，可手动刷新

---

## API 端点

### 新闻接口

| 端点 | 说明 | 示例 |
|------|------|------|
| `/api/news` | 获取新闻列表（可过滤风险等级） | `http://localhost:8080/api/news?level=high` |
| `/api/risk-index` | 获取全局风险指数 | `http://localhost:8080/api/risk-index` |
| `/api/sources` | 获取各新闻源状态 | `http://localhost:8080/api/sources` |
| `/api/refresh` | 强制刷新新闻 | `http://localhost:8080/api/refresh` |

### 风险等级

- **🟢 低危 (Low)** - 风险等级 1-2
- **🟡 中等 (Medium)** - 风险等级 3-5
- **🟠 高危 (High)** - 风险等级 6-8
- **🔴 严重 (Critical)** - 风险等级 9-10

---

## 风险评分规则

### 关键词匹配

每个维度定义一组关键词，命中后根据权重加分：

```python
RISK_KEYWORDS = {
    "bubble":     {"w": 3.0, "words": ["bubble", "valuation", "crash", "崩盘", ...]},
    "regulation": {"w": 2.2, "words": ["regulat", "lawsuit", "antitrust", "调查", ...]},
    "safety":     {"w": 2.6, "words": ["safety", "security", "breach", "风险", ...]},
    "industry":   {"w": 1.8, "words": ["layoff", "job cut", "chip", "裁员", ...]},
}
```

### 严重信号

命中以下关键词时，额外加 4.0 分：
- `ai bubble`, `ai crash`, `market crash`
- `systemic risk`, `catastroph`
- `existential`, `shut down`
- `bankrupt`, `fraud`, `scandal`

### 分级逻辑

```
总分 >= 7.0   → critical
总分 >= 4.5   → high
总分 >= 2.2   → medium
其他         → low
```

---

## 依赖说明

**本项目使用 Python 标准库，无第三方依赖**：
- `http.server` - Web 服务
- `threading` - 并发爬取
- `urllib` - HTTP 请求
- `xml.etree.ElementTree` - RSS 解析
- `json` - 数据序列化

---

## 代码亮点

### 1. 并发新闻抓取

使用线程池并发拉取多个 RSS 源（最大 9 个线程）：

```python
with ThreadPoolExecutor(max_workers=9) as ex:
    futs = {ex.submit(fetch_feed, f): f for f in FEEDS}
    for fut in as_completed(futs):
        arts, st = fut.result()
```

### 2. 智能缓存

- 3 分钟 TTL 自动缓存
- 标题去重（按 80 字符 hash）
- 支持手动强制刷新

### 3. 风险指数聚合

时间衰减权重：
```python
if age < 6 and a["risk_level"] in ("high", "critical"):
    boost += (6 - age) / 6 * 0.4
```

### 4. 静默日志

避免控制台刷屏：

```python
def log_message(self, fmt, *args):
    pass  # 不输出任何日志
```

---

## 守护进程 (watchdog.py)

### 功能

1. **健康检查**：每 120 秒检查 Web 服务响应
2. **自动重启**：服务挂了尝试用 `web_server.py` 重启
3. **日志监控**：扫描最近的 Claude Code 会话日志
4. **错误检测**：识别 GLM 限流、权限拦截、API 错误

### 使用方式

```bash
# 单次检查
python watchdog.py

# 常驻守护
python watchdog.py --loop
```

### 输出示例

```
[10:30:45] Web:OK Claude错误:无 需关注:否
  - Web 服务正常，120 条新闻
```

---

## 常见问题

### Q: 为什么只监控 RSS 源，不直接调用 AI API？

**A**: 为了避免：
- API 限流（OpenAI、Anthropic 等都有速率限制）
- 费用问题（按 token 计费）
- 延迟问题（AI 分析需要时间）
- 简化部署（不需要 API Key）

我们采用**关键词匹配 + 规则评分**的方式，实时性更高，成本为零。

### Q: 数据会持久化吗？

**A**: 当前版本使用内存存储。刷新页面后重新抓取。如需持久化，可以扩展为 SQLite 或其他数据库。

### Q: 如何自定义风险关键词？

**A**: 编辑 `web_server.py` 中的 `RISK_KEYWORDS` 字典：

```python
RISK_KEYWORDS = {
    "your_dim": {
        "w": 2.0,  # 权重
        "label": "你的维度",
        "words": ["keyword1", "keyword2", ...]
    },
}
```

### Q: 服务占用多少资源？

**A**:
- 内存：约 20-50 MB（取决于新闻数量）
- CPU：低（仅 RSS 抓取和关键词匹配）
- 网络：约 100-500 KB/刷新（取决于 RSS 内容）

### Q: 为什么有 .bak_original 目录？

**A**: 这是原始项目的备份。如果需要恢复 Rust 版本，可以：
```bash
cd ai-risk-monitor
cp -r .bak_original src
```

---

## 部署建议

### 本地运行

```bash
python web_server.py
```

### Docker 部署（可选）

创建 `Dockerfile`：

```dockerfile
FROM python:3.11-slim

WORKDIR /app

COPY web_server.py .
COPY templates/ ./templates/

CMD ["python", "web_server.py"]
```

构建和运行：

```bash
docker build -t ai-risk-monitor .
docker run -p 8080:8080 ai-risk-monitor
```

### 生产环境

1. 使用 `watchdog.py --loop` 守护进程
2. 配置日志到文件
3. 使用 Nginx 反向代理
4. 考虑启用 HTTPS

---

## 未来规划

- [ ] SQLite 持久化存储
- [ ] Telegram/Email 告警通知
- [ ] 多用户/权限管理
- [ ] 自定义告警规则
- [ ] 数据导出（CSV、JSON）
- [ ] 更多数据源（GitHub Trending、Reddit 等）

---

## 许可证

MIT License

---

## 相关链接

- [Bloomberg 风格顶部滚动新闻条实现参考](https://github.com/NYTimes/bloomberg-ticker)
- [新闻滚动动画 CSS](https://css-tricks.com/snippets/css/css-news-ticker/)
