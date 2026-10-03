"""Machine verification of agent permission boundaries.

Parses `.opencode/agents/*.md` frontmatter (restricted YAML subset) and
checks the ACTUAL `permission:` mapping against required deny rules, using
the documented OpenCode semantics: shorthand "allow"|"ask"|"deny", or an
object of glob -> action where the LAST matching rule wins. Anything that
cannot be verified fails closed as READ_ONLY_ENFORCEMENT_UNAVAILABLE.
"""
from __future__ import annotations

import fnmatch
from pathlib import Path

SAFE_DISPATCHER_BASH_ALLOWS = (
    "git status --short",
    "git rev-parse HEAD",
    "git rev-parse origin/main",
    "git branch --show-current",
    "git log",
    "git log --oneline --decorate -8",
    "git diff --stat",
    "git diff --stat HEAD",
    "opencode models",
    "python3 scripts/model_orchestrator/__main__.py status",
    "python3 scripts/model_orchestrator/__main__.py doctor",
    "python3 scripts/model_orchestrator/__main__.py models",
    "python3 scripts/model_orchestrator/__main__.py plan",
    "python3 scripts/model_orchestrator/__main__.py step",
    "python3 scripts/model_orchestrator/__main__.py explain",
)

DANGEROUS_DISPATCHER_PROBES = (
    "git branch evil",
    "git branch -D evil",
    "git push origin main",
    "git commit -m x",
    "git checkout main",
    "git switch -c evil",
    "git reset --hard",
    "git restore .",
    "git clean -fd",
    "python3 scripts/model_orchestrator/__main__.py --auto run",
    "python3 scripts/model_orchestrator/__main__.py run --auto",
    "python3 scripts/model_orchestrator/__main__.py resume",
    "python3 scripts/model_orchestrator/__main__.py pause",
    "python3 scripts/model_orchestrator/__main__.py unpause",
    "python3 scripts/model_orchestrator/__main__.py escalate",
    "python3 scripts/agent_supervisor.py --goal checkpoint:9B",
    "opencode run hello",
)


def agent_file(repo: str, name: str) -> Path:
    return Path(repo) / ".opencode" / "agents" / f"{name}.md"


def parse_frontmatter(text: str) -> dict | None:
    """Parse the restricted frontmatter subset. None when malformed."""
    if not text.startswith("---\n"):
        return None
    end = text.find("\n---", 4)
    if end < 0:
        return None
    return _parse_block(text[4:end].splitlines())


def _parse_block(lines: list[str]) -> dict | None:
    root: dict = {}
    stack: list[tuple[int, dict]] = [(-1, root)]
    try:
        for raw in lines:
            if not raw.strip() or raw.strip().startswith("#"):
                continue
            indent = len(raw) - len(raw.lstrip(" "))
            key, sep, value = raw.strip().partition(":")
            if not sep or not key:
                return None
            if len(key) >= 2 and key[0] == key[-1] and key[0] in ("'", '"'):
                key = key[1:-1]  # quoted glob patterns may contain spaces
            elif " " in key:
                return None
            while stack and indent <= stack[-1][0]:
                stack.pop()
            parent = stack[-1][1]
            value = value.strip()
            if value == "":
                child: dict = {}
                parent[key] = child
                stack.append((indent, child))
            elif value in ("allow", "ask", "deny"):
                parent[key] = value
            else:
                parent[key] = value.strip('"').strip("'")
        return root
    except (IndexError, AttributeError):
        return None


def bash_allows(rule: object, command: str) -> str:
    """Effective action for a bash command under documented last-match-wins."""
    if isinstance(rule, str):
        return rule
    if not isinstance(rule, dict):
        return "deny"
    action = "deny"
    for pattern, value in rule.items():
        if fnmatch.fnmatchcase(command, pattern):
            action = value
    return action if action in ("allow", "ask", "deny") else "deny"


def check_dispatcher(repo: str) -> list[str]:
    """Empty list <=> machine-enforced read-only verified."""
    path = agent_file(repo, "model-dispatcher")
    try:
        front = parse_frontmatter(path.read_text(encoding="utf-8"))
    except OSError:
        return ["dispatcher: agent file missing: READ_ONLY_ENFORCEMENT_UNAVAILABLE"]
    if not front:
        return ["dispatcher: frontmatter malformed: READ_ONLY_ENFORCEMENT_UNAVAILABLE"]
    permission = front.get("permission")
    if not isinstance(permission, dict):
        return ["dispatcher: no machine permission block: READ_ONLY_ENFORCEMENT_UNAVAILABLE"]
    problems = []
    if permission.get("edit") != "deny":
        problems.append("dispatcher: edit not denied")
    task = permission.get("task")
    if task != "deny" and not (isinstance(task, dict) and task.get("*") == "deny"):
        problems.append("dispatcher: task/spawn not denied")
    if permission.get("external_directory") != "deny":
        problems.append("dispatcher: external_directory not denied")
    bash = permission.get("bash")
    if bash_allows(bash, "rm -rf /") != "deny":
        problems.append("dispatcher: bash default not deny")
    for allowed in SAFE_DISPATCHER_BASH_ALLOWS:
        if bash_allows(bash, allowed) != "allow":
            problems.append(f"dispatcher: expected allowlist entry missing: {allowed}")
    for probe in DANGEROUS_DISPATCHER_PROBES:
        if bash_allows(bash, probe) == "allow":
            problems.append(f"dispatcher: dangerous command allowed: {probe}")
    if problems:
        return problems + ["READ_ONLY_ENFORCEMENT_UNAVAILABLE"]
    return []


def check_file_rules(repo: str, name: str, required: dict) -> list[str]:
    """Assert an agent file carries exact required permission rules."""
    path = agent_file(repo, name)
    try:
        front = parse_frontmatter(path.read_text(encoding="utf-8"))
    except OSError:
        return [f"{name}: agent file missing"]
    if not front:
        return [f"{name}: frontmatter malformed"]
    permission = front.get("permission")
    if not isinstance(permission, dict):
        return [f"{name}: no permission block"]
    problems = []
    for key, want in required.items():
        got = permission.get(key)
        if isinstance(want, dict):
            if not isinstance(got, dict):
                problems.append(f"{name}: permission.{key} not an object")
                continue
            for pattern, action in want.items():
                if got.get(pattern) != action:
                    problems.append(f"{name}: permission.{key}['{pattern}'] != {action}")
        elif got != want:
            problems.append(f"{name}: permission.{key} != {want}")
    return problems
