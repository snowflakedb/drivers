#!/usr/bin/env python3
"""Generate the Buildkite pipeline YAML for driver test steps.

Runs select_tests.py for each driver, then outputs a pipeline YAML containing
only the driver steps that have relevant changes (non-SKIP). Skipped drivers
are annotated via buildkite-agent but not included in the pipeline.

Usage (from test-selection step on Buildkite):
    python3 ci/generate_buildkite_pipeline.py | buildkite-agent pipeline upload
"""

import os
import subprocess
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
SELECT_SCRIPT = os.path.join(SCRIPT_DIR, "select_tests.py")

DRIVERS = [
    {"name": "rust", "group": "integ,e2e"},
    {"name": "python", "group": "integ,e2e"},
    {"name": "odbc", "group": "integ,e2e"},
    {"name": "java", "group": "e2e"},
]

VAULT_PLUGIN = "${GLOBAL_PLUGIN}/vault_secrets"
DOCKER_IMAGE = (
    "artifactory.ci1.us-west-2.aws-dev.app.snowflake.com/"
    "internal-production-docker-snowflake-virtual/docker/"
    "rhel8-universal-driver-coverage:3"
)

COMMON_STEP = {
    "agents": {"queue": "discovery", "repo": "snowflakedb/universal-driver"},
    "plugins": [
        {
            VAULT_PLUGIN: {
                "secrets": [
                    {
                        "path": "secret/jenkins/rt-tests/driver_validation_parameters_secret",
                        "env_name": "PARAMETERS_SECRET",
                    },
                    {
                        "path": "secret/jenkins/rt-tests/universal_driver_test_suite_api_token",
                        "env_name": "BUILDKITE_ANALYTICS_TOKEN",
                    },
                ]
            }
        },
        {
            "docker#v5.11.0": {
                "image": DOCKER_IMAGE,
                "propagate-environment": True,
                "mount-buildkite-agent": True,
                "environment": ["PARAMETERS_SECRET", "BUILDKITE_ANALYTICS_TOKEN"],
            }
        },
    ],
    "retry": {"automatic": [{"exit_status": "*", "limit": 1}]},
}


def run_test_selection(driver, group):
    """Run select_tests.py and return the filter result."""
    cmd = [
        sys.executable, SELECT_SCRIPT,
        "--driver", driver,
        "--group", group,
        "--verbose",
    ]
    result = subprocess.run(
        cmd,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, universal_newlines=True,
    )
    sys.stderr.write(result.stderr)
    if result.returncode != 0:
        print("ERROR: select_tests.py failed for driver='{}' group='{}' (exit {})".format(
            driver, group, result.returncode), file=sys.stderr)
        sys.exit(result.returncode)
    return result.stdout.strip()


def _is_buildkite():
    return os.environ.get("BUILDKITE") == "true"


def set_metadata(key, value):
    """Store a value in Buildkite meta-data (no-op outside Buildkite)."""
    if not _is_buildkite():
        return
    subprocess.run(
        ["buildkite-agent", "meta-data", "set", key, value],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )


def annotate(message, style="info", context="default"):
    """Add a Buildkite annotation (no-op outside Buildkite)."""
    if not _is_buildkite():
        return
    subprocess.run(
        ["buildkite-agent", "annotate", message, "--style", style,
         "--context", context, "--append"],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    )


RUST_COMMAND = """\
set -euo pipefail
TEST_FILTER=$$(buildkite-agent meta-data get "test-filter-rust")
echo "Filter: $$TEST_FILTER"

./scripts/decode_secrets.sh
yum install -y unzip
export PARAMETER_PATH=/workdir/parameters.json

echo "--- :hammer: Building sf_core"
cargo build --package sf_core
cargo install cargo2junit 2>/dev/null || true

echo "--- :test_tube: Running E2E Tests"
mkdir -p /workdir/junit-results
export RUSTC_BOOTSTRAP=1
if [ "$$TEST_FILTER" = "ALL" ]; then
  cargo test --package sf_core -- --ignored -Z unstable-options --format json --report-time 2>&1 | cargo2junit > /workdir/junit-results/rust-junit.xml
else
  # cargo test uses substring matching — run once per filter and merge JSON streams
  IFS='|' read -ra FILTERS <<< "$$TEST_FILTER"
  for filter in "$${FILTERS[@]}"; do
    echo "Running: cargo test -- --ignored $$filter"
    cargo test --package sf_core -- --ignored "$$filter" -Z unstable-options --format json --report-time 2>&1
  done | cargo2junit > /workdir/junit-results/rust-junit.xml
fi
unset RUSTC_BOOTSTRAP

echo "--- :buildkite: Uploading test results"
buildkite-agent artifact upload "junit-results/*.xml"
buildkite-agent annotate ":white_check_mark: Rust Core -- passed" --style "success" --context "rust-result"
"""

PYTHON_COMMAND = """\
set -euo pipefail
TEST_FILTER=$$(buildkite-agent meta-data get "test-filter-python")
echo "Filter: $$TEST_FILTER"

./scripts/decode_secrets.sh
yum install -y unzip
export PARAMETER_PATH=/workdir/parameters.json

cd python

echo "--- :hammer: Building Python Wheel"
RUSTFLAGS="" hatch build -t wheel
hatch run test.py3.9:install-wheel

echo "--- :test_tube: Running Integ + E2E Tests"
mkdir -p /workdir/junit-results
if [ "$$TEST_FILTER" = "ALL" ]; then
  hatch run test.py3.9:all -- tests/integ/ tests/e2e/ -v --timeout=900 --junitxml=/workdir/junit-results/python-junit.xml
else
  hatch run test.py3.9:all -- $$TEST_FILTER -v --timeout=900 --junitxml=/workdir/junit-results/python-junit.xml
fi

echo "--- :buildkite: Uploading test results"
cd /workdir
buildkite-agent artifact upload "junit-results/*.xml"
buildkite-agent annotate ":white_check_mark: Python -- passed" --style "success" --context "python-result"
"""

ODBC_COMMAND = """\
set -euo pipefail
TEST_FILTER=$$(buildkite-agent meta-data get "test-filter-odbc")
echo "Filter: $$TEST_FILTER"

./scripts/decode_secrets.sh
yum install -y unzip
export PARAMETER_PATH=/workdir/parameters.json
export DRIVER_PATH=/workdir/target/debug/libsfodbc.so

echo "--- :hammer: Building ODBC Driver"
cargo build

echo "--- :hammer: Building C++ Test Suite"
source /opt/rh/gcc-toolset-11/enable
cd odbc_tests
mkdir -p cmake-build
cmake -B cmake-build \\
    -D ODBC_LIBRARY="/usr/lib64/libodbc.so" \\
    -D ODBC_INCLUDE_DIR="/usr/include" \\
    -D DRIVER_TYPE=NEW \\
    .
cmake --build cmake-build -- -j $$(nproc)

echo "--- :test_tube: Running Integ + E2E Tests"
mkdir -p /workdir/junit-results
if [ "$$TEST_FILTER" = "ALL" ]; then
  ctest -j 1 -C Debug --test-dir cmake-build --output-on-failure --no-tests=error -R "e2e|integration" --output-junit /workdir/junit-results/odbc-junit.xml
else
  ctest -j 1 -C Debug --test-dir cmake-build --output-on-failure --no-tests=error -R "$$TEST_FILTER" --output-junit /workdir/junit-results/odbc-junit.xml
fi

echo "--- :buildkite: Uploading test results"
cd /workdir
buildkite-agent artifact upload "junit-results/*.xml"
buildkite-agent annotate ":white_check_mark: ODBC -- passed" --style "success" --context "odbc-result"
"""

JDBC_COMMAND = """\
set -euo pipefail
TEST_FILTER=$$(buildkite-agent meta-data get "test-filter-java")
echo "Filter: $$TEST_FILTER"

./scripts/decode_secrets.sh
yum install -y unzip
export PARAMETER_PATH=/workdir/parameters.json

echo "--- :hammer: Building JDBC Bridge"
cargo build --package sf_core
cargo build --package jdbc_bridge

echo "--- :test_tube: Running Integ + E2E Tests"
export CORE_PATH=/workdir/target/debug/libjdbc_bridge.so
chmod +x jdbc/gradlew
cd jdbc
if [ "$$TEST_FILTER" = "ALL" ]; then
  ./gradlew test --stacktrace
else
  GRADLE_TESTS=""
  IFS='|' read -ra PATTERNS <<< "$$TEST_FILTER"
  for pattern in "$${PATTERNS[@]}"; do
    GRADLE_TESTS="$$GRADLE_TESTS --tests $$pattern"
  done
  ./gradlew test $$GRADLE_TESTS --stacktrace
fi

echo "--- :buildkite: Uploading test results"
mkdir -p /workdir/junit-results
cp build/test-results/test/*.xml /workdir/junit-results/ 2>/dev/null || true
cd /workdir
buildkite-agent artifact upload "junit-results/*.xml"
buildkite-agent annotate ":white_check_mark: JDBC -- passed" --style "success" --context "jdbc-result"
"""

DRIVER_STEPS = {
    "rust": {
        "group_label": ":rust: Rust Core",
        "group_key": "rust-core",
        "step_label": ":rust: Build + E2E Tests",
        "step_key": "rust-core-e2e",
        "timeout": 30,
        "command": RUST_COMMAND,
    },
    "python": {
        "group_label": ":python: Python",
        "group_key": "python",
        "step_label": ":python: Build + Integ + E2E Tests",
        "step_key": "python-integ-e2e",
        "timeout": 45,
        "command": PYTHON_COMMAND,
    },
    "odbc": {
        "group_label": ":c: ODBC",
        "group_key": "odbc",
        "step_label": ":c: Build + Integ + E2E Tests",
        "step_key": "odbc-integ-e2e",
        "timeout": 45,
        "command": ODBC_COMMAND,
    },
    "java": {
        "group_label": ":java: JDBC",
        "group_key": "jdbc",
        "step_label": ":java: Build + Integ + E2E Tests",
        "step_key": "jdbc-integ-e2e",
        "timeout": 45,
        "command": JDBC_COMMAND,
    },
}


def build_step(driver_name):
    """Build a pipeline group dict for a driver."""
    cfg = DRIVER_STEPS[driver_name]
    step = dict(COMMON_STEP)
    step["label"] = cfg["step_label"]
    step["key"] = cfg["step_key"]
    step["timeout_in_minutes"] = cfg["timeout"]
    step["command"] = _LiteralStr(cfg["command"])
    return {
        "group": cfg["group_label"],
        "key": cfg["group_key"],
        "steps": [step],
    }


class _LiteralStr(str):
    """String subclass that PyYAML renders as a literal block scalar (|)."""


def _literal_representer(dumper, data):
    return dumper.represent_scalar("tag:yaml.org,2002:str", data, style="|")


def main():
    try:
        import yaml
    except ImportError:
        print("ERROR: pyyaml is required", file=sys.stderr)
        sys.exit(1)

    yaml.add_representer(_LiteralStr, _literal_representer)

    filters = {}
    skipped = []
    active = []

    print("--- :mag: Running test selection", file=sys.stderr)
    for drv in DRIVERS:
        name = drv["name"]
        result = run_test_selection(name, drv["group"])
        filters[name] = result
        if result == "SKIP":
            skipped.append(name)
        else:
            active.append(name)
        print("  {}: {}".format(name, result), file=sys.stderr)

    lines = []
    for name in active:
        set_metadata("test-filter-" + name, filters[name])
        label = "ALL tests" if filters[name] == "ALL" else "filtered: " + filters[name]
        lines.append(":arrow_forward: {} -- {}".format(name, label))
    if skipped:
        lines.append(":fast_forward: skipped: " + ", ".join(skipped))
    if lines:
        annotate("\n".join(lines), style="info", context="test-selection")

    steps = [build_step(name) for name in active]

    if not active:
        steps = [{"label": ":white_check_mark: All drivers skipped",
                  "command": "echo 'No relevant changes detected'"}]
    else:
        step_keys = [DRIVER_STEPS[name]["step_key"] for name in active]
        steps.append("wait")
        steps.append({
            "label": ":junit: Annotate test results",
            "plugins": [{
                "junit-annotate#v2.4.1": {
                    "artifacts": "junit-results/*.xml",
                    "always-annotate": True,
                }
            }],
            "agents": {"queue": "discovery", "repo": "snowflakedb/universal-driver"},
        })
        steps.append({
            "label": ":bar_chart: Upload to Test Engine",
            "plugins": [
                {
                    VAULT_PLUGIN: {
                        "secrets": [{
                            "path": "secret/jenkins/rt-tests/universal_driver_test_suite_api_token",
                            "env_name": "BUILDKITE_ANALYTICS_TOKEN",
                        }]
                    }
                },
                {
                    "test-collector#v1.10.0": {
                        "files": "junit-results/*.xml",
                        "format": "junit",
                    }
                },
            ],
            "agents": {"queue": "discovery", "repo": "snowflakedb/universal-driver"},
        })

    print(yaml.dump({"steps": steps}, default_flow_style=False, sort_keys=False))


if __name__ == "__main__":
    main()
