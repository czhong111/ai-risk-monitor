#!/usr/bin/env python3
"""监控智谱 API 端点可用性，恢复后写标记文件。每 15 秒探测一次。"""
import json, time, urllib.request, urllib.error, datetime, os, sys

BASE = os.path.dirname(os.path.abspath(__file__))
KEY = None
try:
    cfg = json.load(open(r"C:\Users\Administrator\.claude\settings.json", encoding="utf-8"))
    KEY = cfg["env"]["ANTHROPIC_AUTH_TOKEN"]
except Exception:
    pass

STATUS = os.path.join(BASE, "api_status.json")
LOG = os.path.join(BASE, "api_monitor.log")
ENDPOINT = "https://open.bigmodel.cn/api/anthropic/v1/messages"
MODEL = "glm-4-flash"


def probe():
    if not KEY:
        return "NO_KEY", "配置里读不到 key"
    body = json.dumps({
        "model": MODEL, "max_tokens": 16,
        "messages": [{"role": "user", "content": "OK"}],
    }).encode()
    req = urllib.request.Request(ENDPOINT, data=body, headers={
        "Content-Type": "application/json",
        "x-api-key": KEY,
        "anthropic-version": "2023-06-01",
    })
    try:
        with urllib.request.urlopen(req, timeout=20) as r:
            r.read()
            return "OK", "HTTP %d" % r.status
    except urllib.error.HTTPError as e:
        return "HTTP_%d" % e.code, "HTTP %d" % e.code
    except Exception as e:
        return "NETERR", str(e)[:80]


def log(msg):
    line = "[%s] %s" % (datetime.datetime.now().strftime("%H:%M:%S"), msg)
    with open(LOG, "a", encoding="utf-8") as f:
        f.write(line + "\n")
    print(line, flush=True)


def main():
    consecutive_ok = 0
    log("监控启动，每 15 秒探测 %s" % MODEL)
    while True:
        code, detail = probe()
        if code == "OK":
            consecutive_ok += 1
            log("探测: OK (%s) 连续成功 %d 次" % (detail, consecutive_ok))
        else:
            consecutive_ok = 0
            log("探测: %s (%s)" % (code, detail))
        available = consecutive_ok >= 2
        st = {
            "checked_at": datetime.datetime.now().isoformat(timespec="seconds"),
            "last_code": code,
            "last_detail": detail,
            "consecutive_ok": consecutive_ok,
            "available": available,
            "model": MODEL,
            "hint": "可用，可在 VS Code Claude 里重试" if available else "不可用，等待恢复",
        }
        tmp = STATUS + ".tmp"
        with open(tmp, "w", encoding="utf-8") as f:
            json.dump(st, f, ensure_ascii=False, indent=2)
        os.replace(tmp, STATUS)
        time.sleep(15)


if __name__ == "__main__":
    main()
