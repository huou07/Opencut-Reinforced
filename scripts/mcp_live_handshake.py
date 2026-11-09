#!/usr/bin/env python3
"""Talk to a real `or mcp` server the way an agent host does.

Every other MCP check in this repository runs inside a Rust test. This one is
deliberately outside: it spawns the shipped `or mcp` binary, performs the
JSON-RPC handshake, lists the tools it actually advertises, and calls the ones
that read the live project and a companion tool's provenance record. A failure
here means an agent host could not have connected either.

The server is given a session *pointer*, not a descriptor, because that is what
the application writes for an assistant: the pointer is re-read on every call,
so one command line survives a project switch.

Usage:
  scripts/mcp_live_handshake.py --pointer PATH [--record PATH] [--or PATH]

Exit codes: 0 the handshake and every call succeeded; 1 a call failed or a
response was wrong; 2 the invocation is wrong.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

PROTOCOL_VERSION = "2025-06-18"


class Session:
    def __init__(self, executable: Path, pointer: Path) -> None:
        self.process = subprocess.Popen(
            [str(executable), "mcp", "--pointer", str(pointer)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        self.next_id = 0

    def request(self, method: str, params: dict) -> dict:
        self.next_id += 1
        message = {
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": method,
            "params": params,
        }
        assert self.process.stdin is not None
        assert self.process.stdout is not None
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()
        line = self.process.stdout.readline()
        if not line:
            stderr = self.process.stderr.read() if self.process.stderr else ""
            raise SystemExit(f"or mcp closed the connection: {stderr.strip()}")
        response = json.loads(line)
        if response.get("id") != self.next_id:
            raise SystemExit(f"response id mismatch: {response}")
        if "error" in response:
            raise SystemExit(f"{method} failed: {response['error']}")
        return response["result"]

    def call(self, name: str, arguments: dict | None = None) -> str:
        result = self.request(
            "tools/call", {"name": name, "arguments": arguments or {}}
        )
        text = ""
        for block in result.get("content") or []:
            if block.get("type") == "text":
                text += block.get("text", "")
        if result.get("isError"):
            raise SystemExit(f"tool {name} reported an error: {text.strip()}")
        return text

    def close(self) -> int:
        if self.process.stdin:
            self.process.stdin.close()
        try:
            return self.process.wait(timeout=30)
        except subprocess.TimeoutExpired:
            self.process.kill()
            return 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pointer", required=True, type=Path)
    parser.add_argument(
        "--executable", default=None, type=Path, help="path to the or binary"
    )
    parser.add_argument(
        "--roundtrip",
        default=None,
        type=Path,
        help="output path to record and read back through MCP",
    )
    parser.add_argument(
        "--record",
        default=None,
        type=Path,
        help="provenance sidecar the handshake must read back through MCP",
    )
    arguments = parser.parse_args()

    executable = arguments.executable or (
        Path(__file__).resolve().parent.parent / "target/release/or"
    )
    if not executable.is_file():
        print(f"missing or binary: {executable}", file=sys.stderr)
        return 2
    if not arguments.pointer.is_file():
        print(f"missing session pointer: {arguments.pointer}", file=sys.stderr)
        return 2

    session = Session(executable, arguments.pointer)
    checks: list[str] = []

    hello = session.request(
        "initialize",
        {
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {},
            "clientInfo": {"name": "or-mcp-live-handshake", "version": "1"},
        },
    )
    server = hello.get("serverInfo") or {}
    checks.append(
        f"initialize: server={server.get('name')} version={server.get('version')} "
        f"protocol={hello.get('protocolVersion')}"
    )
    if "tools" not in (hello.get("capabilities") or {}):
        raise SystemExit("the server did not advertise a tools capability")

    listing = session.request("tools/list", {})
    names = [tool["name"] for tool in listing.get("tools") or []]
    checks.append(f"tools/list: {len(names)} tools")
    for required in ("project_status", "tools_list", "tools_provenance_read"):
        if required not in names:
            raise SystemExit(f"the server does not advertise {required}")

    status = json.loads(session.call("project_status"))
    checks.append(
        f"project_status: {status.get('name')!r} "
        f"revision={status.get('project_revision')} dirty={status.get('dirty')}"
    )

    tools = json.loads(session.call("tools_list"))
    ready = [tool for tool in tools.get("tools") or [] if tool.get("state") == "ready"]
    checks.append(
        f"tools_list: {len(ready)} of {len(tools.get('tools') or [])} ready "
        f"({', '.join(tool['id'] for tool in ready) or 'none'})"
    )

    if arguments.roundtrip is not None:
        # Recording a result is the other half of the contract: an agent that
        # just ran a companion tool has to be able to write the record through
        # the same server it reads it from.
        output = arguments.roundtrip
        written = json.loads(
            session.call(
                "tools_provenance_write",
                {
                    "tool": "ffmpeg",
                    "output": str(output),
                    "command": ["ffmpeg", "-i", "in.mov", str(output)],
                    "version": "live-handshake",
                },
            )
        )
        sidecar = Path(str(output) + ".orprovenance.json")
        if not sidecar.is_file():
            raise SystemExit(f"tools_provenance_write did not create {sidecar}")
        reread = json.loads(
            session.call("tools_provenance_read", {"file": str(sidecar)})
        )
        if reread.get("tool_version") != "live-handshake":
            raise SystemExit("the record written through MCP did not read back")
        checks.append(
            f"tools_provenance_write: {written.get('file')} -> "
            f"{reread.get('tool_name')} {reread.get('tool_version')}"
        )
        sidecar.unlink()

    if arguments.record is not None:
        if not arguments.record.is_file():
            raise SystemExit(f"missing provenance record: {arguments.record}")
        expected = json.loads(arguments.record.read_text())
        record = json.loads(
            session.call("tools_provenance_read", {"file": str(arguments.record)})
        )
        checks.append(
            f"tools_provenance_read: {record.get('tool_name')} "
            f"{record.get('tool_version')} produced {record.get('output')}"
        )
        if record.get("tool_id") != expected.get("tool_id"):
            raise SystemExit("the record read through MCP is not the record on disk")
        if record.get("command") != expected.get("command"):
            raise SystemExit("the command read through MCP is not the command on disk")

    for check in checks:
        print(check)
    code = session.close()
    print(f"RESULT: OR_MCP_HANDSHAKE={'PASS' if code == 0 else 'FAIL'}")
    return 0 if code == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
