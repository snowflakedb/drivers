# Override these at command line: make test PYTHON_VERSION=3.12 PYTEST_ARGS="-k test_connection"
PYTHON_VERSION ?= $(shell python3 -c 'import sys;print(f"{sys.version_info.major}.{sys.version_info.minor}")')
# 3.11 -> 311 for tox env names
PYTHON_ID := $(subst .,,$(PYTHON_VERSION))
REPORTS_DIR ?= reports
REFERENCE_DRIVER_VERSION ?= 3.17.2
PYTEST_ARGS ?=
FAIL_ON_REGRESSIONS ?= 0

# Absolute paths to avoid relative path issues across different working directories
DEFAULT_CORE_PATH_MACOS := $(shell cd .. && pwd)/target/debug/libsf_core.dylib
DEFAULT_CORE_PATH_LINUX := $(shell cd .. && pwd)/target/debug/libsf_core.so
DEFAULT_PARAMETER_PATH := $(shell cd .. && pwd)/parameters.json

SHELL := bash
.ONESHELL:  # Execute each recipe in a single shell (needed for multi-line commands)

.PHONY: check-env-or-build check-env-or-fail setup build-wheel test test-local test-local-sequential test-local-tox test-local-tox-sequential test-all-local-tox test-integ-local-tox test-e2e-local-tox test-reference-local-tox test-all ci-test-reference compare compare-local compare-artifacts clean clean-reports run-with-setup run-with-uv run-with-tox


# ===== Reusable Command Components =====

# Platform-aware path detection with environment variable override
DETECTED_CORE_PATH := $(shell \
	if [ -n "$$CORE_PATH" ]; then \
		echo "$$CORE_PATH"; \
	elif [[ "$$(uname)" == "Darwin" ]]; then \
		echo "$(DEFAULT_CORE_PATH_MACOS)"; \
	else \
		echo "$(DEFAULT_CORE_PATH_LINUX)"; \
	fi)

DETECTED_PARAMETER_PATH := $(shell \
	if [ -n "$$PARAMETER_PATH" ]; then \
		echo "$$PARAMETER_PATH"; \
	else \
		echo "$(DEFAULT_PARAMETER_PATH)"; \
	fi)


# Environment variables for local development
LOCAL_ENV = CORE_PATH=$(DETECTED_CORE_PATH) PARAMETER_PATH=$(DETECTED_PARAMETER_PATH)

# Report file paths
UNIVERSAL_REPORT_JSON_PATH = $(REPORTS_DIR)/universal-$(PYTHON_VERSION).json
REFERENCE_REPORT_JSON_PATH = $(REPORTS_DIR)/reference-$(PYTHON_VERSION).json
UNIVERSAL_REPORT_XML_PATH = $(REPORTS_DIR)/universal-$(PYTHON_VERSION).xml
REFERENCE_REPORT_XML_PATH = $(REPORTS_DIR)/reference-$(PYTHON_VERSION).xml

# Common pytest report flags
PYTEST_PARALLEL = -n auto
PYTEST_VERBOSE = -v
PYTEST_REPORT_JSON_FLAGS = --json-report --json-report-file=
PYTEST_REPORT_JSON_UNIVERSAL = $(PYTEST_REPORT_JSON_FLAGS)$(UNIVERSAL_REPORT_JSON_PATH)
PYTEST_REPORT_JSON_REFERENCE = $(PYTEST_REPORT_JSON_FLAGS)$(REFERENCE_REPORT_JSON_PATH)

# Tox environment names
TOX_ALL_ENV = py$(PYTHON_ID)-all
TOX_INTEG_ENV = py$(PYTHON_ID)-integ
TOX_E2E_ENV = py$(PYTHON_ID)-e2e
TOX_REFERENCE_ENV = py$(PYTHON_ID)-reference



# Local development: auto-build missing Rust core
check-env-or-build:
	@echo "Using CORE_PATH: $(DETECTED_CORE_PATH)"
	@echo "Using PARAMETER_PATH: $(DETECTED_PARAMETER_PATH)"
	cd .. && cargo build --package sf_core || { \
		echo "Failed to build Rust core library"; \
		echo "Make sure Rust toolchain is installed: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; \
		exit 1; \
	}
	@if [ ! -f "$(DETECTED_CORE_PATH)" ]; then \
		echo "Error: Core library not found at $(DETECTED_CORE_PATH)"; \
		exit 1; \
	fi


# CI: fail fast if environment not properly set up
check-env-or-fail:
	@if [ -z "$(CORE_PATH)" ]; then \
		echo "Error: CORE_PATH environment variable not set"; \
		exit 1; \
	fi
	@if [ ! -f "$(CORE_PATH)" ]; then \
		echo "Error: Core library not found at $(CORE_PATH)"; \
		exit 1; \
	fi

setup:
	@if ! command -v uv >/dev/null 2>&1; then \
		echo "Installing uv..."; \
		curl -LsSf https://astral.sh/uv/install.sh | sh; \
		echo "uv installed. Please restart your shell or run: source ~/.local/bin/env"; \
		echo "Then run 'make setup' again."; \
		exit 1; \
	else \
		echo "uv already installed: $$(uv --version)"; \
	fi
	uv sync --all-extras
	mkdir -p $(REPORTS_DIR)

build-wheel: setup
	uv build

# ===== Generic Runners =====

run-with-setup: check-env-or-build setup
	$(LOCAL_ENV) $(filter-out $@,$(MAKECMDGOALS))

run-with-uv: check-env-or-build setup
	$(LOCAL_ENV) uv run $(filter-out $@,$(MAKECMDGOALS))

run-with-tox: check-env-or-build setup
	$(LOCAL_ENV) uv run tox -e $(filter-out $@,$(MAKECMDGOALS))

# ===== Local Development =====

test-local: check-env-or-build setup
	$(LOCAL_ENV) uv run pytest tests/ --connector=universal $(PYTEST_PARALLEL) $(PYTEST_VERBOSE) $(PYTEST_ARGS) $(filter-out $@,$(MAKECMDGOALS))

test-local-sequential: check-env-or-build setup
	$(LOCAL_ENV) uv run pytest tests/ --connector=universal $(PYTEST_VERBOSE) $(PYTEST_ARGS) $(filter-out $@,$(MAKECMDGOALS))

test-local-tox: check-env-or-build setup
	$(LOCAL_ENV) uv run tox -e $(TOX_ALL_ENV) -- \
	  $(PYTEST_REPORT_JSON_UNIVERSAL) \
	  $(PYTEST_PARALLEL) $(PYTEST_ARGS) $(filter-out $@ test,$(MAKECMDGOALS))

test-all-local-tox: check-env-or-build setup
	$(LOCAL_ENV) uv run tox -e $(TOX_ALL_ENV) -- \
	  $(PYTEST_REPORT_JSON_UNIVERSAL) \
	  $(PYTEST_PARALLEL) $(PYTEST_ARGS) $(filter-out $@ test,$(MAKECMDGOALS))

test: test-all-local-tox  # Short alias - runs unit, integ, and e2e tests

test-local-tox-sequential: check-env-or-build setup
	$(LOCAL_ENV) uv run tox -e $(TOX_ALL_ENV) -- \
	  $(PYTEST_REPORT_JSON_UNIVERSAL) \
	  $(PYTEST_ARGS) $(filter-out $@,$(MAKECMDGOALS))

test-integ-local-tox: check-env-or-build setup
	$(LOCAL_ENV) uv run tox -e $(TOX_INTEG_ENV) -- \
	  $(PYTEST_REPORT_JSON_UNIVERSAL) \
	  $(PYTEST_PARALLEL) $(PYTEST_ARGS) $(filter-out $@,$(MAKECMDGOALS))

test-e2e-local-tox: check-env-or-build setup
	$(LOCAL_ENV) uv run tox -e $(TOX_E2E_ENV) -- \
	  $(PYTEST_REPORT_JSON_UNIVERSAL) \
	  $(PYTEST_PARALLEL) $(PYTEST_ARGS) $(filter-out $@,$(MAKECMDGOALS))

test-reference-local-tox: check-env-or-build setup
	$(LOCAL_ENV) uv run tox -e $(TOX_REFERENCE_ENV) -- \
	  $(PYTEST_REPORT_JSON_REFERENCE) \
	  $(PYTEST_PARALLEL) $(PYTEST_ARGS) $(filter-out $@,$(MAKECMDGOALS))

# ===== CI Commands =====

ci-test-all: check-env-or-fail
	mkdir -p $(REPORTS_DIR)
	uv run tox -e $(TOX_ALL_ENV) -- \
	  $(PYTEST_REPORT_JSON_UNIVERSAL) \
	  --junitxml=$(UNIVERSAL_REPORT_XML_PATH) \
	  $(PYTEST_ARGS)

ci-test-reference: check-env-or-fail
	mkdir -p $(REPORTS_DIR)
	uv run tox -e $(TOX_REFERENCE_ENV) -- \
	  $(PYTEST_REPORT_JSON_REFERENCE) \
	  --junitxml=$(REFERENCE_REPORT_XML_PATH) \
	  $(PYTEST_ARGS)

# ===== Comparison =====

compare:
	python3 ci/reference_tests/compare_reports.py \
	  --py $(PYTHON_VERSION) \
	  --universal $(UNIVERSAL_REPORT_JSON_PATH) \
	  --reference $(REFERENCE_REPORT_JSON_PATH) \
	  --fail-on-regressions $(FAIL_ON_REGRESSIONS)

compare-local:
	$(MAKE) test-all-local-tox || true  # Continue especially if tests fail
	$(MAKE) test-reference-local-tox || true
	$(MAKE) compare

# For CI: compare downloaded artifacts (paths set via env vars)
ci-compare-artifacts:
	python3 ci/reference_tests/compare_reports.py \
	  --py $(PYTHON_VERSION) \
	  --universal $(UNIVERSAL_TEST_REPORTS_DIR)/universal-$(PYTHON_VERSION).json \
	  --reference $(REFERENCE_TEST_REPORTS_DIR)/reference-$(PYTHON_VERSION).json \
	  --fail-on-regressions $(FAIL_ON_REGRESSIONS)

clean-reports:
	rm -rf $(REPORTS_DIR)

clean: clean-reports # Short alias

# Below: Allow passing pytest args directly after make target
# $(filter-out target-name alias,$(MAKECMDGOALS)) captures trailing args but excludes the target itself
# Without this, "make test -k connection" would pass "test" to pytest, causing it to look for a test named "test"
# The catch-all rule prevents make from trying to execute pytest args as make targets
%:
	@: