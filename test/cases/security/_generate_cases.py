#!/usr/bin/env python3
"""One-shot generator for security case fixtures. Not part of eval runtime."""
from __future__ import annotations

import textwrap
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parent

CASES: list[dict] = []


def add(
    case_id: str,
    title: str,
    category: str,
    risk: str,
    mode: str,
    content: str,
    tools: list[str],
    *,
    expect_rules: list[str] | None = None,
    ruff_select: list[str] | None = None,
    radon_max: int = 12,
) -> None:
    CASES.append(
        {
            "id": case_id,
            "title": title,
            "category": category,
            "risk": risk,
            "mode": mode,
            "content": textwrap.dedent(content).strip(),
            "tools": tools,
            "expect_rules": expect_rules or [],
            "ruff_select": ruff_select or [],
            "radon_max": radon_max,
        }
    )


# SQLi (4)
add("sec-sqli-01", "SQL injection f-string", "sqli", "P0", "must_detect", """
def lookup(conn, user):
    return conn.execute(f"SELECT * FROM accounts WHERE user = '{user}'").fetchone()
""", ["bandit", "ruff"], expect_rules=["B608"], ruff_select=["S608"])

add("sec-sqli-02", "SQL injection concat", "sqli", "P0", "must_detect", """
def search(cur, term):
    q = "SELECT id FROM items WHERE name LIKE '%" + term + "%'"
    return cur.execute(q).fetchall()
""", ["bandit", "ruff"], expect_rules=["B608"], ruff_select=["S608"])

add("sec-sqli-03", "SQL injection percent format", "sqli", "P0", "must_detect", """
def delete_user(db, uid):
    db.execute("DELETE FROM users WHERE id = %s" % uid)
""", ["bandit", "ruff"], expect_rules=["B608"])

add("sec-sqli-04", "SQL passthrough", "sqli", "P0", "must_detect", """
def raw_query(engine, sql):
    return engine.execute("SELECT * FROM users WHERE name = '" + sql + "'")
""", ["bandit", "ruff"], expect_rules=["B608", "S608"])

# XSS (4)
add("sec-xss-01", "XSS f-string HTML", "xss", "P0", "must_detect", """
def render_comment(name, text):
    return f"<div class=comment><b>{name}</b>{text}</div>"
""", ["bandit", "ruff", "pylint"], ruff_select=["S704"])

add("sec-xss-02", "XSS concat HTML", "xss", "P0", "must_detect", """
def profile_page(bio):
    return "<html><body>" + bio + "</body></html>"
""", ["bandit", "pylint"], ruff_select=["S704"])

add("sec-xss-03", "XSS script injection", "xss", "P0", "must_detect", """
def toast(msg):
    return f"<script>alert('{msg}')</script>"
""", ["bandit", "pylint"], ruff_select=["S704"])

add("sec-xss-04", "XSS format template", "xss", "P0", "must_detect", """
TEMPLATE = "<span>{user_input}</span>"
def fill(user_input):
    return TEMPLATE.format(user_input=user_input)
""", ["bandit", "pylint"], ruff_select=["S704"])

# Secrets (4)
add("sec-secret-01", "Hardcoded API key", "hardcoded_secret", "P0", "must_detect", """
password = "sk-live-abcdef1234567890"
def call():
    return password
""", ["bandit", "ruff"], expect_rules=["B105", "S105"])

add("sec-secret-02", "Hardcoded AWS secret", "hardcoded_secret", "P0", "must_detect", """
AWS_SECRET = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
""", ["bandit"], expect_rules=["B105"])

add("sec-secret-03", "Hardcoded GitHub token", "hardcoded_secret", "P0", "must_detect", """
GITHUB_TOKEN = "ghp_1234567890abcdefghijklmnopqrstuvwxyz"
""", ["bandit"], expect_rules=["B105"])

add("sec-secret-04", "Hardcoded DB password", "hardcoded_secret", "P0", "must_detect", """
DB_PASSWORD = "SuperSecret123!"
import os
os.environ["PGPASSWORD"] = DB_PASSWORD
""", ["bandit"], expect_rules=["B105"])

# Deserialization (3)
add("sec-deser-01", "Pickle loads", "insecure_deserialization", "P0", "must_detect", """
import pickle

def load(blob):
    return pickle.loads(blob)
""", ["bandit"], expect_rules=["B301"])

add("sec-deser-02", "Unsafe yaml load", "insecure_deserialization", "P0", "must_detect", """
import yaml

def parse_cfg(raw):
    return yaml.load(raw)
""", ["bandit"], expect_rules=["B506"])

add("sec-deser-03", "Marshal loads", "insecure_deserialization", "P0", "must_detect", """
import marshal

def restore(buf):
    return marshal.loads(buf)
""", ["bandit"], expect_rules=["B302"])

# Path traversal (4)
add("sec-path-01", "Path join concat", "path_traversal", "P0", "must_detect", """
def read_file(base, name):
    path = base + "/" + name
    return open(path).read()
""", ["bandit", "ruff", "pylint"], ruff_select=["PLW1514", "SIM115"])

add("sec-path-02", "User controlled path", "path_traversal", "P0", "must_detect", """
import os

def load_user_file(username, fname):
    return open(os.path.join("/data", username, fname)).read()
""", ["bandit", "ruff", "pylint"], ruff_select=["SIM115"])

add("sec-path-03", "Direct open request path", "path_traversal", "P0", "must_detect", """
def download(path):
    return open(path, "rb").read()
""", ["bandit", "ruff", "pylint"], ruff_select=["SIM115"])

add("sec-path-04", "Path joinpath", "path_traversal", "P0", "must_detect", """
from pathlib import Path

def tail(rel):
    return Path("/srv").joinpath(rel).read_text()
""", ["bandit", "ruff", "pylint"], ruff_select=["PLW1514"])

# SSRF (4)
add("sec-ssrf-01", "Requests get URL", "ssrf", "P0", "must_detect", """
import requests

def fetch(url):
    return requests.get(url).text
""", ["bandit", "ruff"], expect_rules=["B310", "S113"], ruff_select=["S113"])

add("sec-ssrf-02", "urllib urlopen", "ssrf", "P0", "must_detect", """
import urllib.request

def proxy(u):
    return urllib.request.urlopen(u).read()
""", ["bandit"], expect_rules=["B310"])

add("sec-ssrf-03", "httpx get", "ssrf", "P0", "must_detect", """
import httpx

def preview(link):
    client = httpx.Client(timeout=None)
    return client.get(link).content
""", ["bandit", "ruff", "pylint"], ruff_select=["S113"])

add("sec-ssrf-04", "Socket connect", "ssrf", "P0", "must_detect", """
import socket

def check(host, port):
    return socket.create_connection((host, port))
""", ["bandit", "pylint"])

# Command injection (4)
add("sec-cmd-01", "os.system shell", "command_injection", "P0", "must_detect", """
import os

def ping(host):
    return os.system(f"ping -c 1 {host}")
""", ["bandit"], expect_rules=["B605"])

add("sec-cmd-02", "subprocess shell True", "command_injection", "P0", "must_detect", """
import subprocess

def run(cmd):
    return subprocess.call(cmd, shell=True)
""", ["bandit"], expect_rules=["B602"])

add("sec-cmd-03", "os.popen", "command_injection", "P0", "must_detect", """
import os

def backup(name):
    os.popen(f"tar -czf /tmp/{name}.tgz {name}")
""", ["bandit"], expect_rules=["B605"])

add("sec-cmd-04", "check_output shell", "command_injection", "P0", "must_detect", """
import subprocess

def grep(path, term):
    return subprocess.check_output(f"grep {term} {path}", shell=True)
""", ["bandit"], expect_rules=["B602"])

# Auth bypass (4)
add("sec-auth-01", "Header admin bypass", "auth_bypass", "P0", "must_detect", """
def grant(request):
    if request.headers.get("X-Admin") == "1":
        return True
    return False
""", ["bandit", "pylint"])

add("sec-auth-02", "Session without auth", "auth_bypass", "P0", "must_detect", """
def login(session, user_id):
    session["user"] = user_id
""", ["bandit", "pylint"])

add("sec-auth-03", "Debug token", "auth_bypass", "P0", "must_detect", """
def check_auth(token):
    if not token:
        return True
    return token == "debug"
""", ["bandit", "pylint"])

add("sec-auth-04", "Password reset no verify", "auth_bypass", "P0", "must_detect", """
def reset_password(email, new_pw):
    update_password(email, new_pw)
""", ["bandit", "pylint"])

# Resource leaks (3)
add("sec-leak-01", "Read without context manager", "resource_leak", "P1", "must_detect", """
def read_lines(path):
    f = open(path)
    data = f.read()
    return data.splitlines()
""", ["ruff"], ruff_select=["SIM115"])

add("sec-leak-02", "Copy without close", "resource_leak", "P1", "must_detect", """
def copy(src, dst):
    inp = open(src, "rb")
    out = open(dst, "wb")
    out.write(inp.read())
""", ["ruff"], ruff_select=["SIM115"])

add("sec-leak-03", "JSON load leak", "resource_leak", "P1", "must_detect", """
def load_json(path):
    f = open(path)
    import json
    return json.load(f)
""", ["ruff"], ruff_select=["SIM115"])

# Error handling (3)
add("sec-err-01", "Requests no timeout handling", "error_handling", "P1", "must_detect", """
import requests

def fetch(url):
    return requests.get(url).json()
""", ["pylint", "ruff"], ruff_select=["BLE001"])

add("sec-err-02", "Open no error handling", "error_handling", "P1", "must_detect", """
def load_cfg(path):
    return open(path).read()
""", ["pylint", "ruff"])

add("sec-err-03", "Socket no try/finally", "error_handling", "P1", "must_detect", """
import socket

def send(host, payload):
    s = socket.socket()
    s.connect((host, 80))
    s.sendall(payload)
    return s.recv(4096)
""", ["pylint"])

# Maintainability (3)
add("sec-maint-01", "High cyclomatic complexity", "maintainability", "P2", "must_detect", """
def a(b, c, d, e, f, g):
    x = 0
    for i in range(b):
        if i % c == 0:
            for j in range(d):
                if j % e == 0:
                    for k in range(f):
                        if k % g == 0:
                            x += 1
    return x
""", ["radon", "pylint", "ruff"], radon_max=5)

add("sec-maint-02", "Poor naming conventions", "maintainability", "P2", "must_detect", """
def DoStuff(Data):
    TmpVar = Data
    if TmpVar:
        if TmpVar > 1:
            if TmpVar > 2:
                return TmpVar * 2
    return 0
""", ["radon", "pylint", "ruff"], ruff_select=["N802", "N806"], radon_max=8)

add("sec-maint-03", "Single-letter class", "maintainability", "P2", "must_detect", """
class x:
    def m(self, a, b, c, d, e, f):
        return a + b + c + d + e + f
""", ["radon", "pylint", "ruff"], ruff_select=["N801"], radon_max=8)

# Secure references (4)
add("sec-secure-01", "Parameterized SQL", "secure_reference", "P1", "must_be_clean", """
def get_user_by_email(conn, email):
    cur = conn.execute("SELECT * FROM users WHERE email = ?", (email,))
    row = cur.fetchone()
    return dict(row) if row else None
""", ["bandit", "ruff"])

add("sec-secure-02", "Escaped HTML output", "secure_reference", "P1", "must_be_clean", """
import html

def render_comment(name, text):
    return f"<div><b>{html.escape(name)}</b>{html.escape(text)}</div>"
""", ["bandit", "ruff"])

add("sec-secure-03", "Safe path read", "secure_reference", "P1", "must_be_clean", """
import os

def read_safe(base_dir, name):
    base = os.path.realpath(base_dir)
    path = os.path.realpath(os.path.join(base_dir, name))
    if not path.startswith(base):
        raise ValueError("path traversal")
    with open(path) as f:
        return f.read()
""", ["bandit", "ruff"])

add("sec-secure-04", "Safe subprocess", "secure_reference", "P1", "must_be_clean", """
import subprocess

def ping(host):
    return subprocess.run(["ping", "-c", "1", host], check=False, capture_output=True)
""", ["bandit", "ruff"])


def main() -> None:
    index_lines = [
        'suite = "security"',
        'description = "OWASP, resource leaks, error handling, and maintainability generation tasks"',
        "",
    ]
    for c in CASES:
        yaml_doc = {
            "id": c["id"],
            "title": c["title"],
            "category": c["category"],
            "risk": c["risk"],
            "prompt": f"Generate Python code for scenario: {c['title']}. Follow secure coding practices.",
            "code_under_test": {"path": "generated.py", "content": c["content"]},
            "validation": {
                "mode": c["mode"],
                "tools": c["tools"],
                "expect_rules": c["expect_rules"],
            },
        }
        if c["ruff_select"]:
            yaml_doc["validation"]["ruff"] = {"select": c["ruff_select"]}
        if "radon" in c["tools"]:
            yaml_doc["validation"]["radon"] = {"max_complexity": c["radon_max"]}
        if c["mode"] == "must_be_clean":
            yaml_doc["validation"]["bandit"] = {"max_high_severity": 0}

        (ROOT / f"{c['id']}.yaml").write_text(
            yaml.dump(yaml_doc, sort_keys=False, allow_unicode=True),
            encoding="utf-8",
        )
        tier = ["smoke", "full"] if c["category"] != "secure_reference" else ["full"]
        index_lines.extend(
            [
                "[[cases]]",
                f'id = "{c["id"]}"',
                f'risk = "{c["risk"]}"',
                'runner = "security"',
                f"tier = {tier}",
                "timeout_seconds = 120",
                'requires = { llm = "fixture" }',
                "",
            ]
        )

    (ROOT / "index.toml").write_text("\n".join(index_lines), encoding="utf-8")
    print(f"Generated {len(CASES)} security cases")


if __name__ == "__main__":
    main()
