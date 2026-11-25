#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
AI 风险监控 · 本地 Web 服务（纯标准库，无第三方依赖，无二进制）

功能：
  1. 托管 templates/ 静态页面
  2. 拉取多个国际 RSS 新闻源，解析标题/链接/时间
  3. 用 AI 风险关键词库对每条新闻做风险评分，聚合成全球风险指数
  4. 提供 JSON API：/api/news  /api/risk-index  /api/sources  /api/refresh

启动：python web_server.py   然后浏览器打开 http://localhost:8080
"""

import json
import os
import re
import html
import time
import threading
import urllib.request
import urllib.error
import urllib.parse
import xml.etree.ElementTree as ET
from datetime import datetime, timezone, timedelta
from http.server import ThreadingHTTPServer, BaseHTTPRequestHandler
from email.utils import parsedate_to_datetime
from concurrent.futures import ThreadPoolExecutor, as_completed

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
TPL_DIR = os.path.join(BASE_DIR, "templates")
PORT = 8080
CACHE_TTL = 180  # 新闻缓存 3 分钟

# ---------------------------------------------------------------------------
# 新闻源：优先选联通性好、允许服务器抓取的源
# ---------------------------------------------------------------------------
FEEDS = [
    {"name": "Hacker News",     "url": "https://hnrss.org/frontpage",                    "flag": "US"},
    {"name": "HN · AI",          "url": "https://hnrss.org/newest?q=AI",                 "flag": "US"},
    {"name": "TechCrunch",      "url": "https://techcrunch.com/feed/",                   "flag": "US"},
    {"name": "The Verge",       "url": "https://www.theverge.com/rss/index.xml",         "flag": "US"},
    {"name": "Wired",           "url": "https://www.wired.com/feed/rss",                 "flag": "US"},
    {"name": "Ars Technica",    "url": "https://feeds.arstechnica.com/arstechnica/technology-lab", "flag": "US"},
    {"name": "OpenAI News",     "url": "https://openai.com/news/rss.xml",                "flag": "US"},
    {"name": "arXiv · cs.AI",   "url": "https://export.arxiv.org/rss/cs.AI",             "flag": "ACAD"},
    {"name": "NPR",             "url": "https://feeds.npr.org/1001/rss.xml",             "flag": "US"},
]

# ---------------------------------------------------------------------------
# AI 风险关键词库：命中即加分，用于"根据国际新闻推测 AI 风险"
# 四个维度：估值泡沫 / 监管政策 / 安全事件 / 产业冲击（裁员/算力/资本开支）
# ---------------------------------------------------------------------------
RISK_KEYWORDS = {
    "bubble":     {"w": 3.0, "label": "泡沫", "words": [
        "bubble", "overvalued", "valuation", "crash", "selloff", "sell-off", "plunge",
        "correction", "bear market", "inflated", "dot-com", "euphoria", "burn rate",
        "泡沫", "估值", "暴跌", "崩盘", "回调"]},
    "regulation": {"w": 2.2, "label": "监管", "words": [
        "regulat", "lawsuit", "antitrust", "investigation", "ban ", "banned", "restrict",
        "eu ai act", "compliance", "copyright", "probe", "subpoena", "tariff", "sanction",
        "监管", "调查", "诉讼", "禁令", "反垄断", "合规"]},
    "safety":     {"w": 2.6, "label": "安全", "words": [
        "safety", "security", "breach", "leak", "hack", "vulnerab", "incident", "attack",
        "misinformation", "deepfake", "hallucinat", "bias", "jailbreak", "alignment",
        "风险", "泄露", "攻击", "漏洞", "事故", "偏见", "幻觉"]},
    "industry":   {"w": 1.8, "label": "产业", "words": [
        "layoff", "job cut", "cuts jobs", "restructur", "bankrupt", "chip", "gpu", "nvidia",
        "data center", "capex", "capital expenditure", " earnings", "revenue", "ipo",
        "openai", "anthropic", "google deepmind", "meta ai", "xai", "mistral",
        "agentic", "cancel", "fail", "disappoint", "overhyp", "hype", "backlash",
        "裁员", "算力", "芯片", "数据中心", "资本开支", "财报", "项目取消"]},
}

# 高风险信号（直接顶格）
CRITICAL_SIGNALS = [
    "ai bubble", "ai crash", "market crash", "systemic risk", "catastroph",
    "existential", "shut down", "shutdown", "bankrupt", "fraud", "scandal",
]

# ---------------------------------------------------------------------------
# 利好 / 利空 情绪词库：判断一条新闻对 AI 行业与投资者是"有利"还是"有害"
# 与风险评分正交：风险分衡量"波动/不确定性"，情绪分衡量"方向性"
# ---------------------------------------------------------------------------
BENEFICIAL_WORDS = [
    "breakthrough", "launch", "launches", "released", "release", "funding", "raises",
    "raised", "investment", "invests", "record", "surge", "soar", "jump", "growth",
    "grow", "profit", "profitable", "revenue beat", "beats", "upgrade", "open source",
    "partnership", "partners", "adoption", "adopts", "milestone", "win ", "wins",
    "approve", "approved", "boost", "efficiency", "cheaper", "price cut", "advance",
    "innovation", "milestone achievement", "leading model", "state of the art",
    "突破", "融资", "上涨", "增长", "盈利", "开源", "采用", "里程碑",
    "获批", "创新", "利好", "创纪录", "搭载",
]
HARMFUL_WORDS = [
    "bubble", "crash", "plunge", "selloff", "sell-off", "correction", "bear market",
    "lawsuit", "antitrust", "investigation", "probe", "ban ", "banned", "restrict",
    "layoff", "job cut", "cut jobs", "bankrupt", "fraud", "scandal", "breach", "leak",
    "hack", "vulnerab", "incident", "attack", "deepfake", "misinformation",
    "hallucinat", "bias", "jailbreak", "fail", "failure", "disappoint", "overhyp",
    "backlash", "cancel", "warn of", "warning about", "decline", "drop",
    "loss", "miss estimates", "delay", "fine ", "penalty", "shutdown",
    "泡沫", "暴跌", "崩盘", "回调", "裁员", "诉讼", "禁令", "调查", "泄露", "攻击",
    "欺诈", "失败", "下滑", "亏损", "取消", "警告", "利空",
]


def classify_sentiment(title, summary=""):
    """返回 (sentiment, label)：beneficial 有利 / harmful 有害 / neutral 中性"""
    text = (title + " " + summary).lower()
    good = sum(1 for w in BENEFICIAL_WORDS if w in text)
    bad = sum(1 for w in HARMFUL_WORDS if w in text)
    if good > bad:
        return "beneficial", "有利"
    if bad > good:
        return "harmful", "有害"
    return "neutral", "中性"


_cache = {"ts": 0, "articles": [], "sources": []}
_cache_lock = threading.Lock()
_http_headers = {
    "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
                  "(KHTML, like Gecko) Chrome/124.0 Safari/537.36",
    "Accept": "application/rss+xml, application/xml, text/xml, */*",
}


def strip_html(s):
    return html.unescape(re.sub(r"<[^>]+>", "", s or "")).strip()


def _ts_of(iso):
    """ISO 时间字符串转 unix 时间戳（用于排序），失败返回 0"""
    try:
        return datetime.fromisoformat(iso).timestamp()
    except Exception:
        return 0


def parse_date(s):
    if not s:
        return datetime.now(timezone.utc)
    try:
        return parsedate_to_datetime(s).astimezone(timezone.utc)
    except Exception:
        try:
            return datetime.fromisoformat(s.replace("Z", "+00:00")).astimezone(timezone.utc)
        except Exception:
            return datetime.now(timezone.utc)


def score_article(title, summary=""):
    """给单条新闻打风险分，返回 (level, score, tags, dims)"""
    text = (title + " " + summary).lower()
    tags, dims = [], {}
    total = 0.0
    for key, cfg in RISK_KEYWORDS.items():
        hit = 0
        for w in cfg["words"]:
            if w in text:
                hit += 1
        if hit:
            s = min(hit, 3) * cfg["w"]
            dims[key] = round(s, 2)
            total += s
            tags.append(cfg["label"])
    for sig in CRITICAL_SIGNALS:
        if sig in text:
            total += 4.0
            if "严重" not in tags:
                tags.append("严重")
            break
    if total >= 7.0:
        level = "critical"
    elif total >= 4.5:
        level = "high"
    elif total >= 2.2:
        level = "medium"
    else:
        level = "low"
    return level, round(min(total, 10.0), 2), tags, dims


def fetch_feed(feed):
    """抓单个 RSS 源，返回 (articles, status_dict)"""
    req = urllib.request.Request(feed["url"], headers=_http_headers)
    try:
        with urllib.request.urlopen(req, timeout=15) as resp:
            raw = resp.read()
        root = ET.fromstring(raw)
        ns = {"atom": "http://www.w3.org/2005/Atom"}
        items = root.findall(".//item")
        is_atom = False
        if not items:
            items = root.findall(".//atom:entry", ns)
            is_atom = True
        articles = []
        for it in items[:15]:
            def get(tag, atom_tag=None):
                if is_atom:
                    e = it.find("atom:" + (atom_tag or tag), ns)
                    return (e.text or "") if e is not None else ""
                e = it.find(tag)
                return (e.text or "") if e is not None else ""
            title = strip_html(get("title"))
            if not title:
                continue
            link = get("link")
            if is_atom:
                le = it.find("atom:link", ns)
                link = le.attrib.get("href", "") if le is not None else ""
            summary = strip_html(get("description", "summary"))
            # 清洗 HN 等源的噪声字段
            summary = re.split(r"\b(Comments? URL:|Comments?:|Article URL:|Discussion:|https?://\S+\s*#?\s*Comments?|Points:)", summary)[0]
            summary = re.sub(r"https?://\S+", "", summary)
            summary = re.sub(r"\s+", " ", summary).strip()[:300]
            if not summary or len(summary) < 8:
                summary = title
            pub = get("pubDate", "published") or get("updated", "updated")
            level, score, tags, dims = score_article(title, summary)
            sentiment, sentiment_label = classify_sentiment(title, summary)
            # 重要新闻：风险等级 high/critical 或命中严重信号
            important = level in ("high", "critical")
            articles.append({
                "title": title,
                "url": link,
                "summary": summary or title,
                "source": feed["name"],
                "flag": feed["flag"],
                "published": parse_date(pub).isoformat(),
                "risk_level": level,
                "score": score,
                "tags": tags,
                "dims": dims,
                "sentiment": sentiment,
                "sentiment_label": sentiment_label,
                "important": important,
            })
        return articles, {"name": feed["name"], "status": "ok", "count": len(articles)}
    except Exception as e:
        return [], {"name": feed["name"], "status": "error", "error": str(e)[:120]}


def refresh_news(force=False):
    """并发拉取所有源，更新缓存"""
    with _cache_lock:
        if not force and time.time() - _cache["ts"] < CACHE_TTL and _cache["articles"]:
            return
    all_articles, statuses = [], []
    with ThreadPoolExecutor(max_workers=9) as ex:
        futs = {ex.submit(fetch_feed, f): f for f in FEEDS}
        for fut in as_completed(futs):
            arts, st = fut.result()
            all_articles.extend(arts)
            statuses.append(st)
    # 去重（按标题），按时间倒序
    seen, unique = set(), []
    for a in sorted(all_articles, key=lambda x: x["published"], reverse=True):
        key = re.sub(r"\W+", "", a["title"].lower())[:80]
        if key in seen:
            continue
        seen.add(key)
        unique.append(a)
    with _cache_lock:
        _cache["articles"] = unique[:120]
        _cache["sources"] = sorted(statuses, key=lambda x: x["name"])
        _cache["ts"] = time.time()
    print(f"[{datetime.now().strftime('%H:%M:%S')}] 新闻刷新: {len(unique)} 条 / "
          f"{sum(1 for s in statuses if s['status']=='ok')} 源可用")


def compute_index():
    """根据当前新闻聚合全球 AI 风险指数"""
    arts = _cache["articles"]
    if not arts:
        return {"global_score": 0.0, "trend": "stable", "total_events": 0,
                "critical_events": 0, "by_level": {}, "dimensions": {}, "last_updated": ""}
    level_count = {"low": 0, "medium": 0, "high": 0, "critical": 0}
    dim_sum, dim_n = {}, {}
    for a in arts[:60]:
        level_count[a["risk_level"]] = level_count.get(a["risk_level"], 0) + 1
        for d, v in a.get("dims", {}).items():
            dim_sum[d] = dim_sum.get(d, 0) + v
            dim_n[d] = dim_n.get(d, 0) + 1
    weights = {"low": 1, "medium": 3, "high": 6, "critical": 10}
    recent = arts[:30]
    base = sum(weights[a["risk_level"]] for a in recent) / max(len(recent), 1)
    # 时间衰减：最近 6 小时的高风险新闻额外加权
    now = datetime.now(timezone.utc)
    boost = 0.0
    for a in recent:
        try:
            age = (now - datetime.fromisoformat(a["published"])).total_seconds() / 3600
            if age < 6 and a["risk_level"] in ("high", "critical"):
                boost += (6 - age) / 6 * 0.4
        except Exception:
            pass
    score = min(10.0, round(base + boost, 2))
    dims = {k: round(dim_sum[k] / dim_n[k], 2) for k in dim_sum if dim_n.get(k)}
    trend = "increasing" if score >= 6 else ("decreasing" if score <= 3 else "stable")
    return {
        "global_score": score,
        "trend": trend,
        "total_events": len(arts),
        "critical_events": level_count.get("critical", 0),
        "by_level": level_count,
        "dimensions": dims,
        "last_updated": datetime.now(timezone.utc).isoformat(),
    }


# ---------------------------------------------------------------------------
# HTTP 服务
# ---------------------------------------------------------------------------
class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        pass  # 静默，避免刷屏

    def _send(self, data, ctype="application/json; charset=utf-8", status=200):
        if isinstance(data, (dict, list)):
            data = json.dumps(data, ensure_ascii=False).encode("utf-8")
        elif isinstance(data, str):
            data = data.encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        path = self.path.split("?")[0]
        # 新闻列表：/api/news 及 Claude 遗留的 /api/events/latest 别名
        if path in ("/api/news", "/api/events/latest"):
            refresh_news()
            arts = list(_cache["articles"])
            # 重要新闻优先，其次按时间倒序
            arts.sort(key=lambda a: (not a.get("important", False), -_ts_of(a.get("published"))))
            q = self.path.split("?", 1)
            query = q[1] if len(q) > 1 else ""
            level = None
            per_page = None
            m = re.search(r"level=([a-z]+)", query)
            if m:
                level = m.group(1)
            m = re.search(r"per_page=(\d+)", query)
            if m:
                per_page = max(1, min(int(m.group(1)), 200))
            if level:
                arts = [a for a in arts if a["risk_level"] == level]
            if per_page:
                arts = arts[:per_page]
            self._send({"success": True, "data": arts, "count": len(arts)})
        elif path in ("/api/risk-index", "/api/events/risk-index"):
            refresh_news()
            self._send({"success": True, "data": compute_index()})
        elif path in ("/api/sources", "/api/events/sources"):
            refresh_news()
            self._send({"success": True, "data": _cache["sources"]})
        elif path in ("/api/refresh", "/api/events/refresh"):
            refresh_news(force=True)
            self._send({"success": True, "count": len(_cache["articles"])})
        elif path == "/api/translate":
            # 翻译代理：前端 fetch 跨域失败时的同源兜底
            query = self.path.split("?", 1)
            params = urllib.parse.parse_qs(query[1] if len(query) > 1 else "")
            text = (params.get("q") or [""])[0].strip()
            if not text or len(text) > 500:
                self._send({"success": False, "error": "invalid q"}, status=400)
                return
            url = "https://api.mymemory.translated.net/get?" + urllib.parse.urlencode(
                {"q": text, "langpair": "en|zh-CN"})
            try:
                req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
                with urllib.request.urlopen(req, timeout=8) as resp:
                    data = json.loads(resp.read().decode("utf-8"))
                translated = (data.get("responseData") or {}).get("translatedText", "")
                if not translated or translated.lower() == text.lower():
                    matches = data.get("matches") or []
                    translated = matches[0].get("translation", "") if matches else ""
                self._send({"success": bool(translated), "data": translated})
            except Exception as e:
                self._send({"success": False, "error": str(e)[:120]})
        elif path in ("/", "/index.html"):
            self._serve_file(os.path.join(TPL_DIR, "index.html"), "text/html; charset=utf-8")
        elif path in ("/script.js", "/styles.css"):
            name = path.lstrip("/")
            ctype = "application/javascript; charset=utf-8" if name.endswith(".js") else "text/css; charset=utf-8"
            self._serve_file(os.path.join(TPL_DIR, name), ctype)
        else:
            self._send({"error": "not found"}, status=404)

    def do_POST(self):
        path = self.path.split("?")[0]
        if path in ("/api/events/refresh", "/api/refresh"):
            refresh_news(force=True)
            self._send({"success": True, "count": len(_cache["articles"])})
        else:
            self._send({"error": "not found"}, status=404)

    def _serve_file(self, path, ctype):
        try:
            with open(path, "rb") as f:
                data = f.read()
            self.send_response(200)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)
        except FileNotFoundError:
            self._send({"error": "file not found: " + os.path.basename(path)}, status=404)


def main():
    print("=" * 56)
    print("  AI 风险监控 · 本地 Web 服务")
    print("=" * 56)
    print("  正在拉取国际新闻源（首次约需 10-20 秒）...")
    refresh_news(force=True)
    idx = compute_index()
    print(f"  已加载 {len(_cache['articles'])} 条新闻，风险指数 {idx['global_score']}/10")
    print(f"  服务地址: http://localhost:{PORT}")
    print("  按 Ctrl+C 停止")
    print("=" * 56)
    server = ThreadingHTTPServer(("0.0.0.0", PORT), Handler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\n已停止。")
        server.shutdown()


if __name__ == "__main__":
    main()
