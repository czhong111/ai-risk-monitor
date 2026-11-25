#!/usr/bin/env python3
"""守护 hosts：防止 Steam++ 把 GitHub 域名指向 127.0.0.1 导致 VS Code/Claude 连不上 GitHub MCP。

用法：
  python hosts_guard.py          # 单次清理
  python hosts_guard.py --watch  # 常驻，每 20 秒检查一次

原理：把 Steam++ 写入的 `127.0.0.1 <github/huggingface...>` 行注释掉（不删除），
保留 Steam++ 的 Start/End 标记，随时可恢复。
"""
import re, sys, time, datetime

HOSTS = r"C:\Windows\System32\drivers\etc\hosts"
BLOCK_KEYS = ("github", "huggingface", "greasyfork", "hub.docker", "githubusercontent")


def clean():
    try:
        lines = open(HOSTS, encoding="utf-8-sig").read().splitlines()
    except PermissionError:
        return -1, "无权限读 hosts（需管理员）"
    out, n = [], 0
    for ln in lines:
        m = re.match(r"^\s*127\.0\.0\.1\s+(\S+)", ln)
        if m and any(k in m.group(1).lower() for k in BLOCK_KEYS):
            out.append("# [hosts_guard] " + ln)
            n += 1
        else:
            out.append(ln)
    if n:
        try:
            open(HOSTS, "w", encoding="utf-8").write("\n".join(out) + "\n")
        except PermissionError:
            return -1, "无权限写 hosts（需管理员）"
    return n, "ok"


def main():
    watch = "--watch" in sys.argv
    while True:
        n, status = clean()
        ts = datetime.datetime.now().strftime("%H:%M:%S")
        if n < 0:
            print(f"[{ts}] 错误: {status}", flush=True)
        elif n > 0:
            print(f"[{ts}] 已注释 {n} 条 GitHub 屏蔽规则（Steam++ 又写回了）", flush=True)
        else:
            print(f"[{ts}] hosts 干净", flush=True)
        if not watch:
            break
        time.sleep(20)


if __name__ == "__main__":
    main()
