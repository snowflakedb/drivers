#!/usr/bin/env python3
"""Skill eval CI runner for universal-driver.

Runs routing-accuracy evals via claude -p with Cortex env vars.
Uses DB_USER + DB_PASS or DB_PRIVATE_KEY / SNOWHOUSE_PRIVATE_KEY injected by the
Buildkite/WLE agent (ETDP-8162 key-pair migration) for Snowhouse auth — no Vault.

Usage:
    python3 ci/skill_eval_runner.py           # eval changed skills only
    python3 ci/skill_eval_runner.py --all     # eval every skill in the repo
"""

import argparse
import glob
import json
import logging
import os
import platform
import re
import shutil
import ssl
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Any, Callable, Dict, List, Optional, Set
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

try:
    from dataclasses import dataclass, field
except ImportError:
    # Python 3.6: install the backport
    subprocess.run(
        [sys.executable, "-m", "pip", "install", "--quiet", "--user", "dataclasses"],
        check=True,
    )
    from dataclasses import dataclass, field

import yaml

logging.basicConfig(level=logging.INFO, format="%(message)s")
logger = logging.getLogger(__name__)

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

CORTEX_BASE_URL = "https://snowhouse.snowflakecomputing.com/api/v2/cortex/anthropic"
LOGIN_URL = "https://snowhouse.snowflakecomputing.com/session/v1/login-request"
DEFAULT_MODEL = "claude-sonnet-4-6"
NODE_VERSION = "v18.20.7"
SCORE_PASS_THRESHOLD = 0.8
SCORE_WARN_THRESHOLD = 0.6


# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------


@dataclass
class EvalPrompt:
    prompt: str
    notes: Optional[str] = None
    reason: Optional[str] = None


@dataclass
class AccuracyEvalSet:
    should_trigger: List[EvalPrompt]
    should_not_trigger: List[EvalPrompt]


@dataclass
class PromptResult:
    prompt: str
    eval_type: str
    expected_skill: str
    selected_skill: Optional[str]
    reasoning: Optional[str]
    passed: bool
    error: Optional[str] = None


@dataclass
class SkillEvalReport:
    skill_name: str
    skill_dir: str
    eval_kind: str
    total: int
    passed: int
    failed: int
    errors: int
    scored: int
    score: float
    verdict: str
    details: List[PromptResult] = field(default_factory=list)


@dataclass
class ContractConfig:
    enabled_evals: List[str]
    max_prompts_per_eval_ci: int


@dataclass
class ClaudeRunOutput:
    text: str
    api_error: bool = False


# ---------------------------------------------------------------------------
# Contract config
# ---------------------------------------------------------------------------


def load_contract_config():
    # type: () -> ContractConfig
    """Load eval types + prompt cap from sf ai skills contract, or use defaults."""
    sf_bin = shutil.which("sf")
    if sf_bin is None:
        logger.warning("[skill-eval] sf not on PATH, using defaults (routing-accuracy, 25 prompts)")
        return ContractConfig(enabled_evals=["routing-accuracy"], max_prompts_per_eval_ci=25)

    try:
        result = subprocess.run(
            [sf_bin, "ai", "skills", "contract", "--format=json"],
            capture_output=True, text=True, timeout=15,
        )
    except (OSError, subprocess.TimeoutExpired) as e:
        logger.warning("[skill-eval] sf ai skills contract failed (%s), using defaults", e)
        return ContractConfig(enabled_evals=["routing-accuracy"], max_prompts_per_eval_ci=25)

    if result.returncode != 0:
        logger.warning("[skill-eval] sf ai skills contract exited %d, using defaults", result.returncode)
        return ContractConfig(enabled_evals=["routing-accuracy"], max_prompts_per_eval_ci=25)

    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError:
        logger.warning("[skill-eval] sf ai skills contract non-JSON, using defaults")
        return ContractConfig(enabled_evals=["routing-accuracy"], max_prompts_per_eval_ci=25)

    eval_types = data.get("eval_types") or []
    enabled = [et["name"] for et in eval_types if et.get("enabled_in_ci") and et.get("name")]
    max_prompts = next(
        (et["max_prompts_per_eval_ci"] for et in eval_types if et.get("max_prompts_per_eval_ci")),
        25,
    )
    if not enabled:
        enabled = ["routing-accuracy"]
    return ContractConfig(enabled_evals=enabled, max_prompts_per_eval_ci=int(max_prompts))


# ---------------------------------------------------------------------------
# Changed skill detection
# ---------------------------------------------------------------------------


def get_changed_skill_paths(repo_root, base_ref="origin/main"):
    # type: (str, str) -> Optional[List[str]]
    """Return skill dirs touched vs base_ref, or None on git failure."""
    try:
        result = subprocess.run(
            ["git", "diff", "--name-only", "{0}...HEAD".format(base_ref)],
            capture_output=True, text=True, cwd=repo_root,
        )
        if result.returncode != 0:
            return None
    except OSError:
        return None

    skill_dirs = set()  # type: Set[str]
    for fpath in (l.strip() for l in result.stdout.splitlines() if l.strip()):
        if ".claude/" not in fpath:
            continue
        parts = fpath.split("/")
        for i, part in enumerate(parts):
            if part == "skills" and i > 0 and parts[i - 1] == ".claude" and i + 1 < len(parts):
                skill_dirs.add("/".join(parts[: i + 2]))
                break

    return sorted(skill_dirs)


# ---------------------------------------------------------------------------
# Eval candidate discovery
# ---------------------------------------------------------------------------


def _parse_frontmatter(content):
    # type: (str) -> dict
    lines = content.split("\n")
    first = next((i for i, l in enumerate(lines) if l.strip()), -1)
    if first == -1 or lines[first].strip() != "---":
        return {}
    end = next((i for i in range(first + 1, len(lines)) if lines[i].strip() == "---"), -1)
    if end == -1:
        return {}
    try:
        return yaml.safe_load("\n".join(lines[first + 1: end])) or {}
    except Exception:
        return {}


def _extract_skill_name(content):
    # type: (str) -> Optional[str]
    fm = _parse_frontmatter(content)
    return str(fm["name"]) if isinstance(fm, dict) and fm.get("name") else None


def find_eval_candidates(repo_root, changed_dirs, enabled_evals):
    # type: (str, List[str], List[str]) -> List[Tuple[str, str, str, str]]
    """Return (skill_name, skill_dir, eval_type, yaml_path) for each runnable eval."""
    candidates = []
    for skill_dir in changed_dirs:
        skill_md = os.path.join(repo_root, skill_dir, "SKILL.md")
        if not os.path.isfile(skill_md):
            continue
        try:
            with open(skill_md, encoding="utf-8") as f:
                content = f.read()
            skill_name = _extract_skill_name(content) or os.path.basename(skill_dir)
        except OSError:
            skill_name = os.path.basename(skill_dir)

        for eval_type in enabled_evals:
            yaml_path = os.path.join(repo_root, skill_dir, "eval_sets", "{0}.yaml".format(eval_type))
            if os.path.isfile(yaml_path):
                candidates.append((skill_name, skill_dir, eval_type, yaml_path))
    return candidates


# ---------------------------------------------------------------------------
# Eval YAML parsing
# ---------------------------------------------------------------------------


def parse_accuracy_yaml(path):
    # type: (str) -> AccuracyEvalSet
    with open(path, encoding="utf-8") as f:
        data = yaml.safe_load(f)
    if not isinstance(data, dict):
        raise ValueError("{0}: must be a YAML mapping".format(path))

    def _parse(raw, section):
        # type: (List[Any], str) -> List[EvalPrompt]
        out = []
        for i, entry in enumerate(raw):
            if isinstance(entry, str):
                out.append(EvalPrompt(prompt=entry))
            elif isinstance(entry, dict):
                if "prompt" not in entry:
                    raise ValueError("{0}[{1}] missing 'prompt'".format(section, i))
                out.append(EvalPrompt(
                    prompt=str(entry["prompt"]),
                    notes=entry.get("notes"),
                    reason=entry.get("reason"),
                ))
            else:
                raise ValueError("{0}[{1}] must be str or mapping".format(section, i))
        return out

    st = data.get("should_trigger")
    snt = data.get("should_not_trigger")
    if not isinstance(st, list) or not st:
        raise ValueError("{0}: missing non-empty 'should_trigger'".format(path))
    if not isinstance(snt, list) or not snt:
        raise ValueError("{0}: missing non-empty 'should_not_trigger'".format(path))
    return AccuracyEvalSet(
        should_trigger=_parse(st, "should_trigger"),
        should_not_trigger=_parse(snt, "should_not_trigger"),
    )


# ---------------------------------------------------------------------------
# Prompt building + parsing
# ---------------------------------------------------------------------------

# Use concatenation, not str.format() — prompts may contain curly braces.
_ACCURACY_PREFIX = "A developer sends you this message:\n\n\""
_ACCURACY_SUFFIX = '''"

Check what skills are available to you in this session. Which single \
skill would you invoke to help this developer? If no skill is a good \
match, say NONE.

CRITICAL: Do NOT actually invoke any skills. Do NOT use the Skill tool. \
Do NOT take any actions. ONLY report your decision.

RESPOND IN EXACTLY THIS FORMAT:

SELECTED_SKILL: <exact skill name, or NONE>
REASONING: <one sentence explaining your choice>'''

_SELECTED_RE = re.compile(r"SELECTED_SKILL:\s*(.+)", re.IGNORECASE)
_REASONING_RE = re.compile(r"REASONING:\s*(.+)", re.IGNORECASE)


def build_accuracy_prompt(user_prompt):
    # type: (str) -> str
    return _ACCURACY_PREFIX + user_prompt + _ACCURACY_SUFFIX


def parse_selected_skill(output):
    # type: (str) -> Tuple[Optional[str], Optional[str]]
    selected = reasoning = None
    m = _SELECTED_RE.search(output)
    if m:
        raw = m.group(1).strip().strip("`").strip('"').strip("'")
        selected = None if raw.upper() == "NONE" else raw
    m = _REASONING_RE.search(output)
    if m:
        reasoning = m.group(1).strip()
    return selected, reasoning


# ---------------------------------------------------------------------------
# Scoring
# ---------------------------------------------------------------------------


def score_accuracy_prompt(skill_name, eval_type, prompt, selected, reasoning):
    # type: (str, str, str, Optional[str], Optional[str]) -> PromptResult
    if eval_type == "should_trigger":
        passed = selected is not None and selected == skill_name
    else:
        passed = selected is None or selected != skill_name
    return PromptResult(
        prompt=prompt, eval_type=eval_type, expected_skill=skill_name,
        selected_skill=selected, reasoning=reasoning, passed=passed,
    )


def score_skill(skill_name, skill_dir, eval_kind, results):
    # type: (str, str, str, List[PromptResult]) -> SkillEvalReport
    errored = sum(1 for r in results if r.error)
    passed = sum(1 for r in results if r.passed and not r.error)
    failed = sum(1 for r in results if not r.passed and not r.error)
    scored = len(results) - errored
    score = passed / scored if scored > 0 else 0.0
    verdict = "PASS" if score >= SCORE_PASS_THRESHOLD else ("WARN" if score >= SCORE_WARN_THRESHOLD else "FAIL")
    return SkillEvalReport(
        skill_name=skill_name, skill_dir=skill_dir, eval_kind=eval_kind,
        total=len(results), passed=passed, failed=failed, errors=errored,
        scored=scored, score=score, verdict=verdict, details=results,
    )


# ---------------------------------------------------------------------------
# Reporting
# ---------------------------------------------------------------------------


def format_report(reports):
    # type: (List[SkillEvalReport]) -> str
    if not reports:
        return "No skill evals to report."

    lines = [
        "## Skill Eval Results\n",
        "| Skill | Eval | Score | Verdict |",
        "|-------|------|-------|---------|",
    ]
    for r in sorted(reports, key=lambda x: (x.verdict != "FAIL", x.verdict != "WARN", x.skill_name)):
        fraction = "{0}/{1}".format(r.passed, r.scored) + (" +{0}err".format(r.errors) if r.errors else "")
        lines.append("| {0} | {1} | {2:.0%} ({3}) | {4} |".format(
            r.skill_name, r.eval_kind, r.score, fraction, r.verdict))

    failures = [r for r in reports if r.verdict != "PASS"]
    if failures:
        lines.append("\n### Details\n")
        for r in failures:
            lines.append("#### {0} ({1}) — {2}\n".format(r.skill_name, r.eval_kind, r.verdict))
            for d in r.details:
                if not d.passed or d.error:
                    status = "ERROR" if d.error else "FAIL"
                    lines.append("- **{0}** [{1}] `{2}`".format(status, d.eval_type, d.prompt))
                    if d.selected_skill:
                        lines.append("  - Selected: `{0}`".format(d.selected_skill))
                    else:
                        lines.append("  - Selected: NONE")
                    if d.reasoning:
                        lines.append("  - Reasoning: {0}".format(d.reasoning))
                    if d.error:
                        lines.append("  - Error: {0}".format(d.error))
            lines.append("")

    total_pass = sum(1 for r in reports if r.verdict == "PASS")
    total_warn = sum(1 for r in reports if r.verdict == "WARN")
    total_fail = sum(1 for r in reports if r.verdict == "FAIL")
    lines.append("\n**Summary:** {0} passed, {1} warned, {2} failed".format(total_pass, total_warn, total_fail))
    return "\n".join(lines)


# ---------------------------------------------------------------------------
# CWD derivation
# ---------------------------------------------------------------------------


def derive_cwd(skill_dir):
    # type: (str) -> str
    """Return the repo-relative working dir for a skill (prefix before .claude/skills/)."""
    marker = ".claude/skills/"
    idx = skill_dir.find(marker)
    return "" if idx <= 0 else skill_dir[:idx].rstrip("/")


# ---------------------------------------------------------------------------
# claude -p subprocess
# ---------------------------------------------------------------------------


def _run_claude_p(prompt, cwd, env, timeout=30):
    # type: (str, str, Dict[str, str], int) -> ClaudeRunOutput
    cmd = [
        "claude", "-p", prompt,
        "--output-format", "json",
        "--max-turns", "1",
        "--no-session-persistence",
    ]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd, env=env, timeout=timeout)
    except subprocess.TimeoutExpired as e:
        raise RuntimeError("claude -p timed out after {0}s".format(timeout)) from e
    if result.returncode != 0:
        raise RuntimeError("claude -p exited {0}: {1}".format(result.returncode, (result.stderr or "")[:500]))
    try:
        envelope = json.loads(result.stdout)
    except json.JSONDecodeError as e:
        raise RuntimeError("claude -p non-JSON: {0}; stdout: {1}".format(e, (result.stdout or "")[:300])) from e
    text = envelope.get("result", "") if isinstance(envelope, dict) else str(envelope)
    api_error = bool(envelope.get("is_error", False)) if isinstance(envelope, dict) else False
    return ClaudeRunOutput(text=str(text), api_error=api_error)


# ---------------------------------------------------------------------------
# Accuracy eval
# ---------------------------------------------------------------------------


def _truncate_eval_set(eval_set, max_prompts, skill_name):
    # type: (AccuracyEvalSet, int, str) -> AccuracyEvalSet
    total = len(eval_set.should_trigger) + len(eval_set.should_not_trigger)
    if total <= max_prompts:
        return eval_set
    ratio = len(eval_set.should_trigger) / total
    tc = max(1, round(max_prompts * ratio))
    ntc = max(1, max_prompts - tc)
    if tc + ntc > max_prompts:
        tc = max(1, max_prompts - ntc)
    logger.warning("[skill-eval] %s: truncating from %d to %d prompts (CI cap)", skill_name, total, max_prompts)
    return AccuracyEvalSet(
        should_trigger=eval_set.should_trigger[:tc],
        should_not_trigger=eval_set.should_not_trigger[:ntc],
    )


def run_accuracy_eval(skill_name, skill_dir, yaml_path, repo_root, claude_env, max_prompts=None):
    # type: (str, str, str, str, Dict[str, str], Optional[int]) -> SkillEvalReport
    eval_set = parse_accuracy_yaml(yaml_path)
    if max_prompts is not None:
        eval_set = _truncate_eval_set(eval_set, max_prompts, skill_name)

    cwd = os.path.join(repo_root, derive_cwd(skill_dir)) or repo_root
    all_prompts = (
        [(e, "should_trigger") for e in eval_set.should_trigger]
        + [(e, "should_not_trigger") for e in eval_set.should_not_trigger]
    )
    total = len(all_prompts)
    logger.info(
        "[skill-eval] %s: %d prompts (%d should_trigger + %d should_not_trigger, cwd=%s)",
        skill_name, total,
        len(eval_set.should_trigger), len(eval_set.should_not_trigger),
        derive_cwd(skill_dir) or "<root>",
    )

    results = []  # type: List[PromptResult]
    for idx, (entry, eval_type) in enumerate(all_prompts, 1):
        try:
            out = _run_claude_p(build_accuracy_prompt(entry.prompt), cwd, claude_env)
            selected, reasoning = parse_selected_skill(out.text)
            r = score_accuracy_prompt(skill_name, eval_type, entry.prompt, selected, reasoning)
        except RuntimeError as e:
            r = PromptResult(
                prompt=entry.prompt, eval_type=eval_type, expected_skill=skill_name,
                selected_skill=None, reasoning=None, passed=False, error=str(e),
            )
        status = "ERROR" if r.error else ("PASS" if r.passed else "FAIL")
        logger.info(
            "[skill-eval] %s [%d/%d] %s [%s] %r -> %s",
            skill_name, idx, total, status, eval_type,
            entry.prompt[:60], r.selected_skill or "NONE",
        )
        results.append(r)

    report = score_skill(skill_name, skill_dir, "routing-accuracy", results)
    logger.info(
        "[skill-eval] %s: %s (%.0f%%, %d/%d passed)",
        skill_name, report.verdict, report.score * 100, report.passed, report.total,
    )
    return report


EVAL_RUNNERS = {
    "routing-accuracy": run_accuracy_eval,
}  # type: Dict[str, Callable[..., SkillEvalReport]]


# ---------------------------------------------------------------------------
# Auth
# ---------------------------------------------------------------------------


def _make_ssl_context():
    # type: () -> ssl.SSLContext
    # CI pods may lack CA certs — follow the same pattern as other Buildkite runners.
    ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    return ctx


def _snowhouse_login(data):
    # type: (Dict[str, Any]) -> str
    """POST to Snowhouse login-request and return session token."""
    body = json.dumps({"data": data}).encode()
    req = Request(LOGIN_URL, data=body, headers={"Content-Type": "application/json"}, method="POST")
    try:
        with urlopen(req, timeout=30, context=_make_ssl_context()) as resp:
            result = json.loads(resp.read())
    except (HTTPError, URLError, OSError, ValueError) as e:
        raise RuntimeError("Login request failed: {0}".format(e)) from e
    if not result.get("success"):
        raise RuntimeError("Login failed: {0}".format(result.get("message", "unknown")))
    token = result.get("data", {}).get("token", "")  # type: str
    if not token:
        raise RuntimeError("Login response contained empty token")
    return token


def fetch_session_token(user, password):
    # type: (str, str) -> str
    """Exchange DB_USER / DB_PASS for a Snowhouse session token."""
    return _snowhouse_login({
        "CLIENT_APP_ID": "SkillEvalCI",
        "CLIENT_APP_VERSION": "1.0",
        "LOGIN_NAME": user,
        "PASSWORD": password,
        "ACCOUNT_NAME": "snowhouse",
    })


def _b64url(data):
    # type: (bytes) -> str
    import base64
    return base64.urlsafe_b64encode(data).rstrip(b"=").decode("ascii")


def fetch_session_token_keypair(user, private_key_pem):
    # type: (str, str) -> str
    """Exchange DB_USER + RSA private key PEM for a Snowhouse session token (JWT)."""
    try:
        from cryptography.hazmat.primitives import hashes, serialization
        from cryptography.hazmat.primitives.asymmetric import padding
        from cryptography.hazmat.backends import default_backend
        import base64
        import hashlib
        import time as _time
    except ImportError as e:
        raise RuntimeError(
            "cryptography package required for Snowhouse key-pair auth: {0}".format(e)
        ) from e

    pem_bytes = private_key_pem.encode("utf-8") if isinstance(private_key_pem, str) else private_key_pem
    key = serialization.load_pem_private_key(pem_bytes, password=None, backend=default_backend())
    pub_der = key.public_key().public_bytes(
        encoding=serialization.Encoding.DER,
        format=serialization.PublicFormat.SubjectPublicKeyInfo,
    )
    # Snowflake fingerprint is standard base64 (not url-safe) of SHA256(DER public key)
    fp = base64.b64encode(hashlib.sha256(pub_der).digest()).decode("ascii")
    account = "SNOWHOUSE"
    user_u = user.upper()
    now = int(_time.time())
    header = json.dumps({"alg": "RS256", "typ": "JWT"}, separators=(",", ":")).encode()
    claims = json.dumps({
        "iss": "{0}.{1}.SHA256:{2}".format(account, user_u, fp),
        "sub": "{0}.{1}".format(account, user_u),
        "iat": now,
        "exp": now + 60,
    }, separators=(",", ":")).encode()
    signing_input = "{0}.{1}".format(_b64url(header), _b64url(claims))
    signature = key.sign(signing_input.encode("ascii"), padding.PKCS1v15(), hashes.SHA256())
    jwt_token = "{0}.{1}".format(signing_input, _b64url(signature))
    return _snowhouse_login({
        "CLIENT_APP_ID": "SkillEvalCI",
        "CLIENT_APP_VERSION": "1.0",
        "LOGIN_NAME": user,
        "AUTHENTICATOR": "SNOWFLAKE_JWT",
        "TOKEN": jwt_token,
        "ACCOUNT_NAME": "snowhouse",
    })


def _first_env(*keys):
    # type: (*str) -> str
    for k in keys:
        v = os.environ.get(k, "")
        if v:
            return v
    return ""


def build_claude_env(token):
    # type: (str) -> Dict[str, str]
    """Build the subprocess env for claude -p calls."""
    env = {
        "ANTHROPIC_AUTH_TOKEN": 'Snowflake Token="{0}"'.format(token),
        "ANTHROPIC_BASE_URL": CORTEX_BASE_URL,
        "ANTHROPIC_MODEL": DEFAULT_MODEL,
        "DISABLE_AUTOUPDATER": "1",
        "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS": "1",
    }  # type: Dict[str, str]
    allowed_exact = {
        "PATH", "HOME", "USER", "SHELL", "LANG", "TMPDIR", "TMP",
        "HOSTNAME", "PWD", "CI", "HTTP_PROXY", "HTTPS_PROXY", "NO_PROXY",
        "http_proxy", "https_proxy", "no_proxy",
    }
    allowed_prefixes = ("ANTHROPIC_", "CLAUDE_", "BUILDKITE_BUILD_", "BUILDKITE_PIPELINE_", "LC_")
    for key, value in os.environ.items():
        if key in env:
            continue
        if key in allowed_exact or any(key.startswith(p) for p in allowed_prefixes):
            env[key] = value
    return env


# ---------------------------------------------------------------------------
# Node.js + Claude Code installation
# ---------------------------------------------------------------------------


def _node_major_version():
    # type: () -> int
    """Return the major version of the node on PATH, or 0 if not found."""
    try:
        result = subprocess.run(["node", "--version"], capture_output=True, text=True)
        m = re.match(r"v(\d+)", result.stdout.strip())
        return int(m.group(1)) if m else 0
    except OSError:
        return 0


def install_node_and_claude():
    # type: () -> None
    arch = "arm64" if platform.machine() == "aarch64" else "x64"
    node_dir = "/tmp/node-{0}-linux-{1}".format(NODE_VERSION, arch)
    node_bin = "{0}/bin".format(node_dir)

    # claude-code requires Node >=18; workers may have an older system Node.
    if _node_major_version() < 18:
        if not os.path.isdir(node_bin):
            logger.info("--- Installing Node.js %s (%s) ---", NODE_VERSION, arch)
            tarball = "node-{0}-linux-{1}.tar.gz".format(NODE_VERSION, arch)
            subprocess.run(
                "curl -fsSL https://nodejs.org/dist/{0}/{1} | tar -xz -C /tmp".format(NODE_VERSION, tarball),
                shell=True, check=True,
            )
        os.environ["PATH"] = node_bin + os.pathsep + os.environ.get("PATH", "")
        logger.info("Node.js %s active at %s", NODE_VERSION, node_bin)
    else:
        logger.info("Node.js already available (>=18): %s", shutil.which("node"))

    if not shutil.which("claude"):
        logger.info("--- Installing Claude Code ---")
        subprocess.run(["npm", "config", "set", "prefix", os.path.expanduser("~/.local")], check=True)
        subprocess.run(["npm", "install", "-g", "@anthropic-ai/claude-code"], check=True)
        local_bin = os.path.expanduser("~/.local/bin")
        os.environ["PATH"] = local_bin + os.pathsep + os.environ.get("PATH", "")
        claude_json = os.path.expanduser("~/.claude.json")
        if not os.path.exists(claude_json):
            with open(claude_json, "w") as f:
                json.dump({"hasCompletedOnboarding": True}, f)
        logger.info("Claude Code installed")
    else:
        logger.info("Claude Code already available: %s", shutil.which("claude"))


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    # type: () -> int
    parser = argparse.ArgumentParser(description="Run skill routing-accuracy evals")
    parser.add_argument("--all", action="store_true", help="Eval all skills, not just changed ones")
    args = parser.parse_args()

    result = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True,
    )
    repo_root = result.stdout.strip() or os.getcwd()

    # 1. Contract config
    ci_config = load_contract_config()
    logger.info(
        "[skill-eval] Enabled evals: %s, max prompts/eval: %d",
        ci_config.enabled_evals, ci_config.max_prompts_per_eval_ci,
    )

    # 2. Find skills to eval
    if args.all:
        all_paths = glob.glob(os.path.join(repo_root, "**/.claude/skills/*/SKILL.md"), recursive=True)
        changed_dirs = sorted({os.path.relpath(os.path.dirname(p), repo_root) for p in all_paths})
        logger.info("[skill-eval] --all: found %d skills", len(changed_dirs))
    else:
        changed_dirs = get_changed_skill_paths(repo_root)
        if changed_dirs is None:
            logger.warning("[skill-eval] git diff failed, skipping evals")
            return 0
        if not changed_dirs:
            logger.info("[skill-eval] No skill changes detected")
            return 0

    candidates = find_eval_candidates(repo_root, changed_dirs, ci_config.enabled_evals)
    if not candidates:
        logger.info("[skill-eval] No eval sets found for changed skills (run sf ai skills generate-eval to create them)")
        return 0

    logger.info("[skill-eval] %d eval(s) to run:", len(candidates))
    for name, skill_dir, eval_type, _ in candidates:
        logger.info("  %s (%s) — %s", name, eval_type, skill_dir)

    # 3. Install Node.js + Claude Code
    install_node_and_claude()

    if not shutil.which("claude"):
        logger.error("[skill-eval] claude CLI not found after install, skipping evals")
        return 0

    # 4. Auth via DB_USER + key-pair (preferred) or DB_PASS (ETDP-8162)
    db_user = os.environ.get("DB_USER", "") or os.environ.get("APP_JENKINS_USER", "")
    db_pass = os.environ.get("DB_PASS", "")
    db_private_key = _first_env("DB_PRIVATE_KEY", "SNOWHOUSE_PRIVATE_KEY", "APP_JENKINS_PRIVATE_KEY")
    if not db_user or (not db_pass and not db_private_key):
        logger.error(
            "[skill-eval] DB_USER and DB_PASS/DB_PRIVATE_KEY not set — are workers provisioned for Snowhouse auth?"
        )
        return 1

    try:
        if db_private_key:
            logger.info("[skill-eval] Snowhouse auth via key-pair (user=%s)", db_user)
            token = fetch_session_token_keypair(db_user, db_private_key)
        else:
            logger.info("[skill-eval] Snowhouse auth via password (user=%s)", db_user)
            token = fetch_session_token(db_user, db_pass)
    except RuntimeError as e:
        logger.error("[skill-eval] Snowhouse auth failed: %s", e)
        return 1

    claude_env = build_claude_env(token)

    # 5. Run evals (parallel by skill)
    reports = []  # type: List[SkillEvalReport]
    with ThreadPoolExecutor(max_workers=3) as pool:
        futures = {}
        for name, skill_dir, eval_type, yaml_path in candidates:
            runner_fn = EVAL_RUNNERS.get(eval_type)
            if runner_fn is None:
                logger.warning("[skill-eval] No runner for eval type %r, skipping", eval_type)
                continue
            fut = pool.submit(
                runner_fn, name, skill_dir, yaml_path, repo_root, claude_env,
                ci_config.max_prompts_per_eval_ci,
            )
            futures[fut] = (name, eval_type)

        for fut in as_completed(futures):
            name, eval_type = futures[fut]
            try:
                reports.append(fut.result())
            except Exception as e:
                logger.exception("[skill-eval] %s/%s raised an unexpected error", name, eval_type)
                reports.append(SkillEvalReport(
                    skill_name=name, skill_dir="", eval_kind=eval_type,
                    total=0, passed=0, failed=0, errors=1, scored=0, score=0.0, verdict="FAIL",
                ))

    # 6. Report
    print(format_report(reports))

    return 1 if any(r.verdict == "FAIL" for r in reports) else 0


if __name__ == "__main__":
    sys.exit(main())
