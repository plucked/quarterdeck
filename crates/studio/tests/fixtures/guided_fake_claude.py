#!/usr/bin/env python3
"""Offline fake CLI exercises the real per-run HTTP endpoint and stream contract."""
import json
import os
from pathlib import Path
import sys
import subprocess
import time
import urllib.request

arguments = sys.argv[1:]
Path("arguments.json").write_text(json.dumps(arguments))
Path("endpoint.txt").write_text(os.environ["QUARTERDECK_MCP_URL"])
Path("prompt.txt").write_text(sys.stdin.read())
mode = Path("mode.txt").read_text().strip()


def emit(event):
    print(json.dumps(event), flush=True)


emit({"type": "system", "subtype": "init", "model": "offline-opus"})
if mode in ["cancel", "cancel_descendant"]:
    if mode == "cancel_descendant":
        subprocess.Popen(["sleep", "30"])
    time.sleep(30)
    sys.exit(1)
if mode == "login":
    print("Please log in", file=sys.stderr)
    sys.exit(1)

sequence = 0


def call(name, arguments):
    global sequence
    sequence += 1
    body = json.dumps({"jsonrpc": "2.0", "id": sequence, "method": "tools/call", "params": {"name": name, "arguments": arguments, "_meta": {"io.modelcontextprotocol/protocolVersion":"2026-07-28", "io.modelcontextprotocol/clientInfo":{"name":"offline-guided-test","version":"1"}, "io.modelcontextprotocol/clientCapabilities":{}}}}).encode()
    request = urllib.request.Request(os.environ["QUARTERDECK_MCP_URL"], data=body, headers={
        "Content-Type": "application/json", "Accept": "application/json, text/event-stream",
        "Authorization": "Bearer " + os.environ["QUARTERDECK_MCP_TOKEN"],
        "MCP-Protocol-Version": "2026-07-28", "Mcp-Method": "tools/call", "Mcp-Name": name,
    })
    with urllib.request.urlopen(request, timeout=5) as response:
        envelope = json.load(response)
    assert "error" not in envelope, envelope
    result = envelope["result"]
    emit({"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": str(sequence), "is_error": result.get("isError", False), "content": result["content"]}]}})
    return result["structuredContent"]


context = call("review_context", {})
scope = {"run_id": context["run_id"], "snapshot_id": context["snapshot_id"]}
if mode == "no_finish":
    receipt = "invented-receipt"
else:
    candidate = {"label": "Code", "description": "Read the changed behavior.", "suggest_viewed": False,
                 "items": [{"label": "Whole folder", "description": "Read code.", "paths": ["src/"]}, {"label": "Duplicate", "description": "Read lib.", "paths": ["src/lib.rs"]}]}
    rejected = call("guide_replace_bucket", dict(scope, bucket_id="code", expected_revision=0, operation_id="overlap", candidate=candidate))
    assert rejected["status"] == "rejected"
    assert rejected["issues"][0]["code"] == "duplicate_file_in_bucket"
    candidate["items"] = [{"label": "Code", "description": "Read the behavior.", "paths": ["src/lib.rs"]}]
    accepted = call("guide_replace_bucket", dict(scope, bucket_id="code", expected_revision=0, operation_id="code", candidate=candidate))
    assert accepted["revision"] == 1
    rejected = call("guide_validate", dict(scope, expected_revision=1, ordered_bucket_ids=["code"]))
    assert rejected["issues"][0]["code"] == "missing_paths"
    candidate["items"].append({"label": "Docs", "description": "Compare the documented behavior.", "paths": ["README.md"]})
    accepted = call("guide_replace_bucket", dict(scope, bucket_id="code", expected_revision=1, operation_id="complete", candidate=candidate))
    assert accepted["revision"] == 2
    ready = call("guide_finish", dict(scope, expected_revision=2, ordered_bucket_ids=["code"], operation_id="finish"))
    assert ready["status"] == "ready"
    receipt = ready["validation_receipt"]
    Path("finished.txt").write_text("sealed")
    if mode == "wrong_receipt":
        receipt = "different-receipt"

terminal = {"type": "result", "subtype": "success", "is_error": False,
            "structured_output": {"validation_receipt": receipt}, "total_cost_usd": 0.0, "session_id": "offline-session"}
if mode == "failed_terminal":
    terminal["is_error"] = True
    terminal["subtype"] = "error_during_execution"
    terminal["result"] = "Synthetic terminal failure"
if mode == "redaction":
    emit({"type":"system", "credential":os.environ["QUARTERDECK_MCP_TOKEN"], "endpoint":os.environ["QUARTERDECK_MCP_URL"]})
if mode == "inherited_stderr":
    subprocess.Popen(["sleep", "30"], stdout=subprocess.DEVNULL)
emit(terminal)
if mode == "output_limit":
    print("x" * (32 * 1024 * 1024 + 1), flush=True)
if mode == "duplicate_terminal":
    emit(terminal)
if mode == "failed_exit":
    sys.exit(7)
