// ============ 全局状态 ============
let allNews = [];
let currentFilter = 'all';
let tickerItems = [];
let tickerOffset = 0;
let tickerPaused = false;
let tickerRAF = null;
let lastTickerWidth = 0;

const riskLabels = { low: '低危', medium: '中等', high: '高危', critical: '严重' };
const dimLabels = { bubble: '估值泡沫', regulation: '监管政策', safety: '安全风险', industry: '产业冲击' };
const dimColors = { bubble: '#f97316', regulation: '#a78bfa', safety: '#ef4444', industry: '#38bdf8' };

// ============ 初始化 ============
document.addEventListener('DOMContentLoaded', () => {
    loadAll();
    setupEventListeners();
    setInterval(loadAll, 180000); // 3 分钟刷新，与后端缓存一致
});

function setupEventListeners() {
    document.querySelectorAll('.filter-btn').forEach(btn => {
        btn.addEventListener('click', () => {
            document.querySelectorAll('.filter-btn').forEach(b => b.classList.remove('active'));
            btn.classList.add('active');
            currentFilter = btn.dataset.filter;
            renderNews();
        });
    });
    const pauseBtn = document.getElementById('ticker-pause');
    pauseBtn.addEventListener('click', () => {
        tickerPaused = !tickerPaused;
        pauseBtn.textContent = tickerPaused ? '▶' : '❚❚';
    });
    document.getElementById('refresh-btn').addEventListener('click', async () => {
        const btn = document.getElementById('refresh-btn');
        btn.textContent = '↻ 刷新中…';
        try { await fetch('/api/events/refresh', { method: 'POST' }); await loadAll(); }
        finally { btn.textContent = '↻ 刷新'; }
    });
}

// ============ 数据加载 ============
async function loadAll() {
    try {
        const [newsRes, idxRes, srcRes] = await Promise.all([
            fetch('/api/events/latest?per_page=100'),
            fetch('/api/risk-index'),
            fetch('/api/sources')
        ]);
        const news = await newsRes.json();
        const idx = await idxRes.json();
        const src = await srcRes.json();
        if (news.success) {
            allNews = news.data;
            renderNews();
            renderTicker();
            document.getElementById('news-count').textContent = `(${allNews.length})`;
        }
        if (idx.success) renderIndex(idx.data);
        if (src.success) renderSources(src.data);
    } catch (e) {
        console.error('加载失败:', e);
    }
}

// ============ 渲染新闻列表 ============
function renderNews() {
    const container = document.getElementById('events-container');
    const list = currentFilter === 'all' ? allNews : allNews.filter(n => n.risk_level === currentFilter);
    if (!list.length) {
        container.innerHTML = '<div class="loading-card">该等级暂无新闻</div>';
        return;
    }
    container.innerHTML = list.slice(0, 60).map(n => {
        const tags = n.tags.map(t => `<span class="tag dim">${esc(t)}</span>`).join('');
        const url = n.url ? `href="${escAttr(n.url)}" target="_blank" rel="noopener"` : '';
        const score = n.score > 0 ? `<span class="tag ${n.risk_level}">风险 ${n.score}</span>` : '';
        return `
        <div class="event-card ${n.risk_level}">
            <h3><a ${url}>${esc(n.title)}</a></h3>
            <p class="summary">${esc(n.summary)}</p>
            <div class="meta">
                <span class="tag ${n.risk_level}">${riskLabels[n.risk_level] || n.risk_level}</span>
                ${score}
                <span>📰 ${esc(n.source)}</span>
                <span>🕐 ${fmtTime(n.published)}</span>
                ${tags}
            </div>
        </div>`;
    }).join('');
}

// ============ 渲染风险指数与维度 ============
function renderIndex(idx) {
    const scoreEl = document.getElementById('global-score');
    scoreEl.textContent = idx.global_score != null ? idx.global_score.toFixed(1) : '--';
    scoreEl.style.color = scoreColor(idx.global_score);
    const trendMap = { increasing: '↑ 上升中', decreasing: '↓ 下降中', stable: '→ 平稳' };
    document.getElementById('trend').textContent = (trendMap[idx.trend] || idx.trend) +
        (idx.last_updated ? ` · ${fmtTime(idx.last_updated)}` : '');
    document.getElementById('total-events').textContent = idx.total_events ?? 0;
    document.getElementById('critical-events').textContent = idx.critical_events ?? 0;
    document.getElementById('high-events').textContent = idx.by_level?.high ?? 0;
    document.getElementById('last-update').textContent = idx.last_updated ? fmtTime(idx.last_updated) : '--';

    const dimEl = document.getElementById('dimensions');
    const dims = idx.dimensions || {};
    const keys = Object.keys(dims);
    if (!keys.length) { dimEl.innerHTML = '<div class="dim-loading">暂无数据</div>'; return; }
    dimEl.innerHTML = keys.map(k => {
        const v = Math.min(dims[k], 10);
        return `<div class="dim-row">
            <span class="dim-name">${dimLabels[k] || k}</span>
            <div class="dim-bar-wrap"><div class="dim-bar" style="width:${v*10}%;background:${dimColors[k] || '#64748b'}"></div></div>
            <span class="dim-val">${v.toFixed(1)}</span>
        </div>`;
    }).join('');
}

function renderSources(sources) {
    const el = document.getElementById('sources-container');
    el.innerHTML = sources.map(s => `
        <div class="source-row">
            <span class="source-name">${esc(s.name)}</span>
            <span class="source-status">
                <span class="status-dot ${s.status === 'ok' ? '' : 'err'}"></span>
                ${s.status === 'ok' ? s.count + ' 条' : '不可用'}
            </span>
        </div>`).join('');
}

// ============ Bloomberg 风格无缝滚动 ============
function renderTicker() {
    if (!allNews.length) return;
    const track = document.getElementById('ticker-track');
    const items = allNews.slice(0, 30);
    const html = items.map(n => `
        <a class="ticker-item lv-${n.risk_level}" ${n.url ? `href="${escAttr(n.url)}" target="_blank" rel="noopener"` : ''}>
            <span class="flag">${esc(n.flag || 'NEWS')}</span>
            <span class="t-title">${esc(n.title)}</span>
            ${n.risk_level === 'critical' || n.risk_level === 'high' ? `<span class="sep">▲</span>` : ''}
        </a>`).join('');
    // 复制两份实现无缝循环
    track.innerHTML = html + html;
    if (tickerRAF) cancelAnimationFrame(tickerRAF);
    tickerOffset = 0;
    // 等布局完成后测量半幅宽度
    requestAnimationFrame(() => {
        lastTickerWidth = track.scrollWidth / 2;
        tickerRAF = requestAnimationFrame(animateTicker);
    });
}

function animateTicker() {
    if (!tickerPaused && lastTickerWidth > 0) {
        tickerOffset -= 0.6; // 滚动速度 px/帧
        if (-tickerOffset >= lastTickerWidth) tickerOffset = 0;
        document.getElementById('ticker-track').style.transform = `translateX(${tickerOffset}px)`;
    }
    tickerRAF = requestAnimationFrame(animateTicker);
}

// ============ 工具函数 ============
function fmtTime(iso) {
    if (!iso) return '--';
    try {
        const d = new Date(iso);
        const diff = (Date.now() - d.getTime()) / 1000;
        if (diff < 60) return '刚刚';
        if (diff < 3600) return Math.floor(diff / 60) + ' 分钟前';
        if (diff < 86400) return Math.floor(diff / 3600) + ' 小时前';
        return d.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' }) + ' ' +
               d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
    } catch { return '--'; }
}

function scoreColor(s) {
    if (s == null) return '#8a97ab';
    if (s >= 6) return '#ef4444';
    if (s >= 4) return '#f97316';
    if (s >= 2) return '#f59e0b';
    return '#00d0a8';
}

function esc(s) {
    return String(s ?? '').replace(/[&<>"']/g, c =>
        ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}
function escAttr(s) { return esc(s).replace(/`/g, '&#96;'); }
