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
const sentiLabels = { beneficial: '有利', harmful: '有害', neutral: '中性' };

// ============ 翻译（悬浮触发，带缓存） ============
const translateCache = new Map();   // 原文 -> 译文（含 null 表示失败，避免反复重试）
const translatePending = new Map(); // 原文 -> Promise，防止并发重复请求
let hoverTimer = null;

/**
 * 判断文本是否已经是中文为主（无需翻译）。
 * 统计 CJK 字符占比，超过 25% 视为中文。
 */
function isMostlyChinese(text) {
  if (!text) return true;
  const cjk = (text.match(/[\u4e00-\u9fff]/g) || []).length;
  return cjk / text.length > 0.25;
}

/**
 * 调用 MyMemory 免费翻译 API（无需 key，支持中英互译）。
 * 国内可访问；失败返回 null。
 */
async function fetchTranslation(text) {
  if (translateCache.has(text)) return translateCache.get(text);
  if (translatePending.has(text)) return translatePending.get(text);

  const task = (async () => {
    try {
      const url = 'https://api.mymemory.translated.net/get?q=' +
        encodeURIComponent(text) + '&langpair=en|zh-CN';
      const res = await fetch(url, { signal: AbortSignal.timeout(8000) });
      if (!res.ok) throw new Error('HTTP ' + res.status);
      const data = await res.json();
      let translated = data?.responseData?.translatedText || '';
      // MyMemory 有时返回整段大写或原样，做基础清理
      if (!translated || translated.toLowerCase() === text.toLowerCase()) {
        translated = data?.matches?.[0]?.translation || '';
      }
      translated = translated
        .replace(/&quot;/g, '"').replace(/&#39;/g, "'")
        .replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>')
        .trim();
      const result = translated && translated.toLowerCase() !== text.toLowerCase()
        ? translated : null;
      translateCache.set(text, result);
      return result;
    } catch (e) {
      translateCache.set(text, null); // 失败也缓存，避免反复请求
      return null;
    } finally {
      translatePending.delete(text);
    }
  })();

  translatePending.set(text, task);
  return task;
}

/**
 * 在标题下方插入/更新翻译气泡。
 * 事件委托绑定在容器上，避免每次渲染重新绑定。
 */
async function handleTitleHover(anchor) {
  const title = anchor.textContent.trim();
  if (!title || isMostlyChinese(title)) return;

  const card = anchor.closest('.event-card');
  if (!card) return;

  // 若已存在气泡则直接返回（成功或失败都保留，避免抖动）
  let bubble = card.querySelector('.zh-translation');
  if (bubble) return;

  bubble = document.createElement('span');
  bubble.className = 'zh-translation pending';
  bubble.innerHTML = '<span class="zh-label">中文</span>翻译中…';
  anchor.insertAdjacentElement('afterend', bubble);

  const translated = await fetchTranslation(title);
  if (!bubble.isConnected) return; // 卡片已被重绘
  if (translated) {
    bubble.innerHTML = '<span class="zh-label">中文</span>' + esc(translated);
    bubble.classList.remove('pending');
  } else {
    bubble.innerHTML = '<span class="zh-label">中文</span>翻译暂不可用（可点击标题查看原文）';
    bubble.classList.remove('pending');
  }
}

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
  if (pauseBtn) pauseBtn.addEventListener('click', () => {
    tickerPaused = !tickerPaused;
    pauseBtn.textContent = tickerPaused ? '▶' : '❚❚';
  });
  const refreshBtn = document.getElementById('refresh-btn');
  if (refreshBtn) refreshBtn.addEventListener('click', async () => {
    refreshBtn.textContent = '↻ 刷新中…';
    try { await fetch('/api/refresh', { method: 'POST' }); await loadAll(); }
    finally { refreshBtn.textContent = '↻ 刷新'; }
  });

  // 悬浮翻译：事件委托，覆盖所有新闻标题（含筛选后重绘的卡片）
  const container = document.getElementById('events-container');
  if (container) {
    container.addEventListener('mouseover', (e) => {
      const anchor = e.target.closest('.event-card h3 a');
      if (!anchor) return;
      clearTimeout(hoverTimer);
      hoverTimer = setTimeout(() => handleTitleHover(anchor), 350);
    });
    container.addEventListener('mouseout', (e) => {
      const anchor = e.target.closest('.event-card h3 a');
      if (!anchor) return;
      clearTimeout(hoverTimer); // 移出则取消未触发的翻译
    });
  }
}

// ============ 数据加载 ============
async function loadAll() {
  try {
    const [newsRes, idxRes, srcRes] = await Promise.all([
      fetch('/api/news?per_page=150'),
      fetch('/api/risk-index'),
      fetch('/api/sources')
    ]);
    const news = await newsRes.json();
    const idx = await idxRes.json();
    const src = await srcRes.json();
    if (news.success) {
      allNews = news.data || [];
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
  let list = currentFilter === 'all' ? allNews : allNews.filter(n => {
    if (currentFilter === 'important') return n.important;
    return n.risk_level === currentFilter;
  });
  if (!list.length) {
    container.innerHTML = '<div class="loading-card">该分类暂无新闻</div>';
    return;
  }
  container.innerHTML = list.slice(0, 80).map(n => {
    const tags = (n.tags || []).map(t => `<span class="tag dim">${esc(t)}</span>`).join('');
    const url = n.url ? `href="${escAttr(n.url)}" target="_blank" rel="noopener"` : '';
    const score = n.score > 0 ? `<span class="tag ${n.risk_level}">风险 ${n.score}</span>` : '';
    const s = n.sentiment || 'neutral';
    const benefit = `<span class="benefit-tag ${s === 'beneficial' ? 'positive' : s === 'harmful' ? 'negative' : ''}">${s === 'beneficial' ? '▲' : s === 'harmful' ? '▼' : '—'} ${sentiLabels[s] || '中性'}</span>`;
    const importantBadge = n.important ? '<span class="important-flag">🔥 重要</span>' : '';
    // 卡片 class：风险等级 + 情绪方向
    const cardClass = ['event-card', n.risk_level, s === 'beneficial' ? 'good' : s === 'harmful' ? 'bad' : ''].filter(Boolean).join(' ');
    // 滚动条友好：给标题加原生 title 属性作为翻译兜底提示
    const titleHint = isMostlyChinese(n.title) ? '' : ` title="悬浮查看中文翻译"`;
    return `
    <div class="${cardClass}">
        <h3><a ${url}${titleHint}>${esc(n.title)}</a>${importantBadge}</h3>
        <p class="summary">${esc(n.summary)}</p>
        <div class="meta">
            ${benefit}
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
  if (!scoreEl) return;
  scoreEl.textContent = idx.global_score != null ? idx.global_score.toFixed(1) : '--';
  scoreEl.style.color = scoreColor(idx.global_score);
  const trendMap = { increasing: '↑ 上升中', decreasing: '↓ 下降中', stable: '→ 平稳' };
  const trendEl = document.getElementById('trend');
  if (trendEl) trendEl.textContent = (trendMap[idx.trend] || idx.trend) +
    (idx.last_updated ? ` · ${fmtTime(idx.last_updated)}` : '');
  const set = (id, v) => { const e = document.getElementById(id); if (e) e.textContent = v; };
  set('total-events', idx.total_events ?? 0);
  set('critical-events', idx.critical_events ?? 0);
  set('high-events', idx.by_level?.high ?? 0);
  set('last-update', idx.last_updated ? fmtTime(idx.last_updated) : '--');

  const dimEl = document.getElementById('dimensions');
  if (!dimEl) return;
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
  if (!el) return;
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
  if (!track) return;
  // 重要新闻优先 + 情绪标记，取前 40 条
  const items = allNews.slice(0, 40);
  const html = items.map(n => {
    const s = n.sentiment || 'neutral';
    const mark = s === 'beneficial' ? '<span class="t-beni">▲</span>' :
                 s === 'harmful' ? '<span class="t-harm">▼</span>' : '';
    const imp = n.important ? '<span class="t-imp">🔥</span>' : '';
    // 滚动条标题：中文为主时不加翻译提示
    const zhHint = isMostlyChinese(n.title) ? n.title :
      n.title + ' （悬浮标题看中文翻译）';
    return `<a class="ticker-item lv-${n.risk_level} s-${s}" title="${escAttr(zhHint)}" ${n.url ? `href="${escAttr(n.url)}" target="_blank" rel="noopener"` : ''}>
        <span class="flag">${esc(n.flag || 'NEWS')}</span>${mark}${imp}
        <span class="t-title">${esc(n.title)}</span>
    </a>`;
  }).join('');
  // 复制两份实现无缝循环
  track.innerHTML = html + html;
  if (tickerRAF) cancelAnimationFrame(tickerRAF);
  tickerOffset = 0;
  requestAnimationFrame(() => {
    lastTickerWidth = track.scrollWidth / 2;
    tickerRAF = requestAnimationFrame(animateTicker);
  });
}

function animateTicker() {
  if (!tickerPaused && lastTickerWidth > 0) {
    tickerOffset -= 0.6; // 滚动速度 px/帧
    if (-tickerOffset >= lastTickerWidth) tickerOffset = 0;
    const track = document.getElementById('ticker-track');
    if (track) track.style.transform = `translateX(${tickerOffset}px)`;
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
  if (s >= 6) return '#dc2626';
  if (s >= 4) return '#f97316';
  if (s >= 2) return '#eab308';
  return '#16a34a';
}

function esc(s) {
  return String(s ?? '').replace(/[&<>"']/g, c =>
    ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}
function escAttr(s) { return esc(s).replace(/`/g, '&#96;'); }
