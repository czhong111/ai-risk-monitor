#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
守护与监督脚本（纯标准库）

用途：
  1. 检查本地 Web 服务 http://localhost:8080 是否存活，挂了尝试自动拉起
  2. 扫描 VS Code 里 Claude Code 的会话日志，识别 GLM 限流 / 权限拦截 / API 错误
  3. 把状态写入 status.json，供自动化任务读取后决定是否通知用户

用法：
  python watchdog.py            # 单次检查
  python watchdog.py --loop     # 常驻，每 2 分钟检查一次
"""

import os
import sys
import json
import time
import glob
import subprocess
import urllib.request
from datetime import datetime

BASE = os.path.dirname(os.path.abspath(__file__))
STATUS_FILE = os.path.join(BASE, "status.json")
CLAUDE_PROJ = r"C:\Users\Administrator\.claude\projects\c--Users-Administrator-AppData-Roaming-Code-User"
HEALTH_URL = "http://localhost:8080/api/news"

# 需要人工/代理介入的错误特征
ERROR_PATTERNS = {
    "glm_rate_limit": ["529", "temporarily unavailable", "访问量过大", "overloaded", "timed out", "繁忙", "retrying error"],
    "permission_denied": ["Permission for this action was denied", "classifier", "auto mode cannot determine"],
    "api_error": ["API Error", "529", "500", "502", "503"],
}


def check_web_service():
    """返回 (alive, detail)"""
    try:
        req = urllib.request.Request(HEALTH_URL, headers={"User-Agent": "watchdog"})
        with urllib.request.urlopen(req, timeout=8) as r:
            data = json.loads(r.read().decode("utf-8"))
            return True, f"正常，{data.get('count', '?')} 条新闻"
    except Exception as e:
        return False, str(e)[:120]


def try_start_service():
    """尝试后台拉起 web_server.py"""
    try:
        py = sys.executable
        subprocess.Popen(
            [py, "-u", os.path.join(BASE, "web_server.py")],
            cwd=BASE,
            creationflags=subprocess.CREATE_NEW_PROCESS_GROUP | 0x00000008,  # DETACHED_PROCESS
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        time.sleep(12)
        alive, detail = check_web_service()
        return alive, detail
    except Exception as e:
        return False, str(e)[:120]


def scan_claude_logs():
    """扫描最近的 Claude 会话日志，返回错误摘要"""
    result = {"last_activity": None, "errors": [], "needs_attention": False}
    if not os.path.isdir(CLAUDE_PROJ):
        result["errors"].append({"type": "log_dir_missing", "detail": CLAUDE_PROJ})
        return result
    logs = sorted(glob.glob(os.path.join(CLAUDE_PROJ, "*.jsonl")),
                  key=os.path.getmtime, reverse=True)
    if not logs:
        return result
    latest = logs[0]
    result["last_activity"] = datetime.fromtimestamp(os.path.getmtime(latest)).isoformat()
    result["latest_log"] = os.path.basename(latest)
    # 只读最后 200KB，避免大文件
    size = os.path.getsize(latest)
    with open(latest, "rb") as f:
        if size > 200 * 1024:
            f.seek(size - 200 * 1024)
        tail = f.read().decode("utf-8", errors="replace")
    found = set()
    for etype, pats in ERROR_PATTERNS.items():
        for p in pats:
            if p.lower() in tail.lower():
                found.add(etype)
                break
    # 统计最新一条是否还在 Wrangling / thinking
    if "Wrangling" in tail[-4000:] or '"type":"assistant"' in tail[-2000:]:
        found.add("possibly_stuck")
    for etype in sorted(found):
        result["errors"].append({"type": etype})
    # 限流或权限问题需要关注
    result["needs_attention"] = bool(
        {"glm_rate_limit", "permission_denied", "api_error"} & found
    )
    return result


def run_check():
    alive, detail = check_web_service()
    actions = []
    if not alive:
        ok, d = try_start_service()
        actions.append(f"Web 服务未响应，尝试拉起：{'成功' if ok else '失败'}（{d}）")
        alive, detail = ok, d

    logs = scan_claude_logs()
    status = {
        "checked_at": datetime.now().isoformat(),
        "web_service": {"alive": alive, "detail": detail},
        "claude": logs,
        "actions": actions,
        "needs_attention": logs.get("needs_attention", False) or not alive,
    }
    with open(STATUS_FILE, "w", encoding="utf-8") as f:
        json.dump(status, f, ensure_ascii=False, indent=2)
    return status


def main():
    loop = "--loop" in sys.argv
    while True:
        s = run_check()
        print(f"[{s['checked_at'][11:19]}] Web:{'OK' if s['web_service']['alive'] else 'DOWN'} "
              f"Claude错误:{','.join(e['type'] for e in s['claude']['errors']) or '无'} "
              f"需关注:{'是' if s['needs_attention'] else '否'}")
        for a in s["actions"]:
            print("  -", a)
        if not loop:
            break
        time.sleep(120)


if __name__ == "__main__":
    main()
