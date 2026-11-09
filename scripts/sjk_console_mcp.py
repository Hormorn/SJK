#!/usr/bin/env python3
"""MCP server that lets an AI assistant look at a running SJK client.

It talks to the client through `cl_consoleSocket` (docs/client.md#console-socket):
a loopback TCP line protocol that streams the console and runs the commands sent.
The port and password are read from the client's config.cfg, so the only setup is
in the game:

    cl_consoleSocketPassword <something>
    cl_consoleSocket 29071

Register it with Claude Code (once, local to this checkout):

    claude mcp add -s local sjk-client -- python S:/Git/SJK/scripts/sjk_console_mcp.py

Tools:
  console      run one console command, return the lines it printed
  console_log  the console lines seen since the bridge connected, optionally filtered
  screenshot   take a screenshot (screenshotJPEG) and return it as an image

Commands that would speak or act for the player on a server (chat, votes, rcon,
connect/disconnect, movement, binds, exec/vstr, admin commands) are refused: the
bridge is for looking and for local settings. Standard library plus Pillow
(optional, to shrink screenshots). Stdio transport, JSON-RPC lines.
"""

from __future__ import annotations

import base64
import io
import json
import os
import re
import socket
import sys
import threading
import time
from collections import deque
from pathlib import Path

DEFAULT_GAME_DATA = r"C:\Program Files (x86)\Steam\steamapps\common\Jedi Academy\GameData"
PROTOCOL_VERSION = "2024-11-05"
LOG_KEEP = 5000
SCREENSHOT_MAX_WIDTH = 1600

# First words refused outright (lower case).
BLOCKED = {
    "say", "say_team", "tell", "vsay", "vsay_team", "vtell", "whisper", "w", "r", "reply",
    "rcon", "callvote", "callteamvote", "vote", "teamvote",
    "connect", "reconnect", "disconnect", "quit", "exit", "kill", "team", "name",
    "exec", "execq", "vstr", "bind", "unbind", "unbindall", "alias", "writeconfig",
    "cmd", "forcecmd",
}
# First-word prefixes refused: chat variants, JA+/jaPRO admin commands.
BLOCKED_PREFIXES = ("say", "tell", "am", "admin")
# Cvars that may not be set: identity, the socket itself, passwords.
BLOCKED_CVARS = re.compile(r"^(name|password|rconpassword|cl_consolesocket.*|cl_hub.*|cl_identity.*)$", re.I)
COLOUR = re.compile(r"\^[0-9a-zA-Z]")


def game_data() -> Path:
    return Path(os.environ.get("JKA_GAME_DATA") or DEFAULT_GAME_DATA)


def config_value(text: str, name: str) -> str | None:
    match = re.search(rf'^\s*seta?\s+{re.escape(name)}\s+"([^"]*)"', text, re.I | re.M)
    return match.group(1) if match else None


def refusal(command: str) -> str | None:
    if any(c in command for c in ";\r\n"):
        return "one command at a time: ';' and line breaks are refused"
    words = command.split()
    if not words:
        return "empty command"
    first = words[0].lstrip("/\\").lower()
    if first.startswith(("+", "-")):
        return "movement and action commands are refused"
    if first in BLOCKED or first.startswith(BLOCKED_PREFIXES):
        return f"'{first}' acts for you on the server or runs other commands; type it yourself"
    if first in {"set", "seta", "sets", "setu", "toggle", "reset"} and len(words) > 1:
        if BLOCKED_CVARS.match(words[1]):
            return f"'{words[1]}' is not changed through the bridge"
    return None


class Client:
    """One authenticated connection to cl_consoleSocket, read on a thread."""

    def __init__(self) -> None:
        self.sock: socket.socket | None = None
        self.lines: deque[tuple[int, str]] = deque(maxlen=LOG_KEEP)
        self.count = 0
        self.lock = threading.Condition()

    def connect(self) -> None:
        if self.sock:
            return
        config = game_data() / "SJK" / "config.cfg"
        try:
            text = config.read_text(encoding="utf-8", errors="replace")
        except OSError as error:
            raise RuntimeError(f"cannot read {config}: {error}") from error
        port = int(os.environ.get("SJK_CONSOLE_PORT") or config_value(text, "cl_consoleSocket") or 0)
        password = os.environ.get("SJK_CONSOLE_PASSWORD") or config_value(text, "cl_consoleSocketPassword") or ""
        if port <= 0 or not password:
            raise RuntimeError(
                "the console socket is off. In the game: cl_consoleSocketPassword <something>, "
                "then cl_consoleSocket 29071"
            )
        try:
            sock = socket.create_connection(("127.0.0.1", port), timeout=3)
        except OSError as error:
            raise RuntimeError(f"no client listening on 127.0.0.1:{port} ({error}); is the game running?") from error
        sock.sendall(password.encode("cp1252", "replace") + b"\n")
        sock.settimeout(None)
        self.sock = sock
        threading.Thread(target=self._read, args=(sock,), daemon=True).start()

    def _read(self, sock: socket.socket) -> None:
        pending = b""
        try:
            while chunk := sock.recv(65536):
                pending += chunk
                *done, pending = pending.split(b"\n")
                with self.lock:
                    for raw in done:
                        line = COLOUR.sub("", raw.decode("utf-8", "strict") if _utf8(raw) else raw.decode("cp1252", "replace"))
                        line = line.replace("\x19", "").rstrip("\r")
                        self.count += 1
                        self.lines.append((self.count, line))
                    self.lock.notify_all()
        except OSError:
            pass
        with self.lock:
            if self.sock is sock:
                self.sock = None
            self.lines.append((self.count + 1, "[bridge] disconnected from the client"))
            self.count += 1
            self.lock.notify_all()

    def run(self, command: str, wait: float) -> list[str]:
        self.connect()
        with self.lock:
            start = self.count
        assert self.sock
        try:
            self.sock.sendall(command.encode("cp1252", "replace") + b"\n")
        except OSError as error:
            self.sock = None
            raise RuntimeError(f"lost the client: {error}") from error
        deadline = time.monotonic() + wait
        quiet_after = 0.25  # stop early once output has been quiet this long
        last = time.monotonic()
        with self.lock:
            seen = self.count
            while (now := time.monotonic()) < deadline:
                if self.count != seen:
                    seen, last = self.count, now
                elif seen != start and now - last > quiet_after:
                    break
                self.lock.wait(min(0.05, deadline - now))
            return [line for number, line in self.lines if number > start]

    def log(self, count: int, pattern: str | None) -> list[str]:
        self.connect()
        regex = re.compile(pattern, re.I) if pattern else None
        with self.lock:
            lines = [line for _, line in self.lines if not regex or regex.search(line)]
        return lines[-count:]


def _utf8(raw: bytes) -> bool:
    try:
        raw.decode("utf-8")
        return any(b >= 0x80 for b in raw)
    except UnicodeDecodeError:
        return False


CLIENT = Client()

TOOLS = [
    {
        "name": "console",
        "description": (
            "Run one console command in the running SJK client and return the console lines printed "
            "while waiting (cvar values, cvarlist, r_drawTimings, perfmark, status, etc.). Chat, votes, "
            "rcon, connect/disconnect, binds, exec/vstr, movement and admin commands are refused. "
            "Changing graphics cvars is allowed but tell the user what you changed."
        ),
        "inputSchema": {
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "One console command, no ';'."},
                "wait_ms": {"type": "integer", "description": "How long to collect output, 100-10000 (default 600)."},
            },
            "required": ["command"],
        },
    },
    {
        "name": "console_log",
        "description": "Console lines received since the bridge connected (newest last), optionally filtered by a regex.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "count": {"type": "integer", "description": "Lines to return (default 100, max 2000)."},
                "match": {"type": "string", "description": "Case-insensitive regex to keep only matching lines."},
            },
        },
    },
    {
        "name": "screenshot",
        "description": "Take a screenshot of what the client shows now and return it as an image (shrunk to 1600 px wide).",
        "inputSchema": {"type": "object", "properties": {}},
    },
]


def screenshot() -> list[dict]:
    name = time.strftime("mcp_%Y%m%d_%H%M%S")
    folder = game_data() / "SJK" / "screenshots"
    path = folder / f"{name}.jpg"
    output = CLIENT.run(f"screenshotJPEG {name}", 0.3)
    deadline = time.monotonic() + 8
    size = -1
    while time.monotonic() < deadline:
        if path.exists():
            now = path.stat().st_size
            if now > 0 and now == size:
                break
            size = now
        time.sleep(0.2)
    else:
        raise RuntimeError(f"no screenshot appeared at {path}; console said: {output[-5:]}")
    data = path.read_bytes()
    try:
        from PIL import Image

        image = Image.open(io.BytesIO(data))
        if image.width > SCREENSHOT_MAX_WIDTH:
            image = image.resize((SCREENSHOT_MAX_WIDTH, round(image.height * SCREENSHOT_MAX_WIDTH / image.width)))
            buffer = io.BytesIO()
            image.convert("RGB").save(buffer, "JPEG", quality=85)
            data = buffer.getvalue()
    except ImportError:
        pass
    return [
        {"type": "image", "data": base64.b64encode(data).decode(), "mimeType": "image/jpeg"},
        {"type": "text", "text": f"saved as {path}"},
    ]


def call(name: str, arguments: dict) -> list[dict]:
    if name == "console":
        command = str(arguments.get("command", "")).strip()
        if reason := refusal(command):
            raise RuntimeError(f"refused: {reason}")
        wait = max(100, min(10000, int(arguments.get("wait_ms", 600)))) / 1000
        lines = CLIENT.run(command, wait)
        return [{"type": "text", "text": "\n".join(lines) or "(no output)"}]
    if name == "console_log":
        count = max(1, min(2000, int(arguments.get("count", 100))))
        lines = CLIENT.log(count, arguments.get("match"))
        return [{"type": "text", "text": "\n".join(lines) or "(nothing yet)"}]
    if name == "screenshot":
        return screenshot()
    raise RuntimeError(f"unknown tool {name}")


def handle(message: dict) -> dict | None:
    method, ident = message.get("method"), message.get("id")
    if ident is None:
        return None  # notification
    if method == "initialize":
        result = {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "sjk-client", "version": "1"},
        }
    elif method == "tools/list":
        result = {"tools": TOOLS}
    elif method == "tools/call":
        params = message.get("params") or {}
        try:
            result = {"content": call(params.get("name", ""), params.get("arguments") or {})}
        except Exception as error:  # reported to the model, not a protocol error
            result = {"content": [{"type": "text", "text": str(error)}], "isError": True}
    elif method == "ping":
        result = {}
    else:
        return {"jsonrpc": "2.0", "id": ident, "error": {"code": -32601, "message": f"unknown method {method}"}}
    return {"jsonrpc": "2.0", "id": ident, "result": result}


def main() -> None:
    for line in sys.stdin:
        if not line.strip():
            continue
        try:
            reply = handle(json.loads(line))
        except json.JSONDecodeError:
            reply = {"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": "parse error"}}
        if reply is not None:
            sys.stdout.write(json.dumps(reply) + "\n")
            sys.stdout.flush()


if __name__ == "__main__":
    main()
