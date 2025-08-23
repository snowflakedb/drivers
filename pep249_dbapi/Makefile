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

.PHONY: check-env-or-build check-env-or-fail setup test test-local tox test-local-tox test-reference-local test-reference-local-tox test-all-universal test-integ-universal test-integ-reference compare compare-artifacts compare-local clean-reports

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

# Local development: auto-build missing Rust core
check-env-or-build:
	@echo "Using CORE_PATH: $(DETECTED_CORE_PATH)"
	@echo "Using PARAMETER_PATH: $(DETECTED_PARAMETER_PATH)"
	@if [ ! -f "$(DETECTED_CORE_PATH)" ]; then \
		echo "Rust core library not found, building..."; \
		cd .. && cargo build --package sf_core || { \
			echo "Failed to build Rust core library"; \
			echo "Make sure Rust toolchain is installed: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"; \
			exit 1; \
		}; \
		echo "Rust core library built successfully"; \
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
	uv sync --extra dev
	mkdir -p $(REPORTS_DIR)

# ===== Local Development =====

test-local: check-env-or-build setup
	CORE_PATH=$(DETECTED_CORE_PATH) PARAMETER_PATH=$(DETECTED_PARAMETER_PATH) uv run pytest tests/ --connector=universal -n auto -v $(PYTEST_ARGS) $(filter-out test-local test,$(MAKECMDGOALS))

test: test-local  # Short alias

test-local-tox: check-env-or-build setup
	CORE_PATH=$(DETECTED_CORE_PATH) PARAMETER_PATH=$(DETECTED_PARAMETER_PATH) uv run tox -e py$(PYTHON_ID)-all-universal -- \
	  --json-report --json-report-file=$(REPORTS_DIR)/universal-$(PYTHON_VERSION).json \
	  -n auto $(PYTEST_ARGS) $(filter-out test-local-tox tox,$(MAKECMDGOALS))

tox: test-local-tox  # Short alias

test-reference-local: check-env-or-build setup
	CORE_PATH=$(DETECTED_CORE_PATH) PARAMETER_PATH=$(DETECTED_PARAMETER_PATH) uv run pytest tests/integ/ --connector=reference -v -n auto \
	  --json-report --json-report-file=$(REPORTS_DIR)/reference-$(PYTHON_VERSION).json $(PYTEST_ARGS) $(filter-out test-reference-local,$(MAKECMDGOALS))

test-reference-local-tox: check-env-or-build setup
	CORE_PATH=$(DETECTED_CORE_PATH) PARAMETER_PATH=$(DETECTED_PARAMETER_PATH) uv run tox -e py$(PYTHON_ID)-integ-reference -- \
	  --json-report --json-report-file=$(REPORTS_DIR)/reference-$(PYTHON_VERSION).json \
	  -n auto $(PYTEST_ARGS)

# ===== CI Commands =====

test-all-universal: check-env-or-fail
	mkdir -p $(REPORTS_DIR)
	uv run tox -e py$(PYTHON_ID)-all-universal -- \
	  --json-report --json-report-file=$(REPORTS_DIR)/universal-$(PYTHON_VERSION).json \
	  --junitxml=$(REPORTS_DIR)/universal-$(PYTHON_VERSION).xml \
	  $(PYTEST_ARGS)

test-integ-reference: check-env-or-fail
	mkdir -p $(REPORTS_DIR)
	uv run tox -e py$(PYTHON_ID)-integ-reference -- \
	  --json-report --json-report-file=$(REPORTS_DIR)/reference-$(PYTHON_VERSION).json \
	  --junitxml=$(REPORTS_DIR)/reference-$(PYTHON_VERSION).xml \
	  $(PYTEST_ARGS)

# ===== Comparison =====

compare:
	python3 ci/reference_tests/compare_reports.py \
	  --py $(PYTHON_VERSION) \
	  --universal $(REPORTS_DIR)/universal-$(PYTHON_VERSION).json \
	  --reference $(REPORTS_DIR)/reference-$(PYTHON_VERSION).json \
	  --fail-on-regressions $(FAIL_ON_REGRESSIONS)

compare-local:
	$(MAKE) test-local-tox || true  # Continue even if tests fail - we want the comparison
	$(MAKE) test-reference-local-tox || true
	$(MAKE) compare

# For CI: compare downloaded artifacts (paths set via env vars)
compare-artifacts:
	python3 ci/reference_tests/compare_reports.py \
	  --py $(PYTHON_VERSION) \
	  --universal $(UNIVERSAL_TEST_REPORTS_DIR)/universal-$(PYTHON_VERSION).json \
	  --reference $(REFERENCE_TEST_REPORTS_DIR)/reference-$(PYTHON_VERSION).json \
	  --fail-on-regressions $(FAIL_ON_REGRESSIONS)

clean-reports:
	rm -rf $(REPORTS_DIR)

# Magic: Allow passing pytest args directly after make target
# $(filter-out target-name alias,$(MAKECMDGOALS)) captures trailing args but excludes the target itself
# Without this, "make test -k connection" would pass "test" to pytest, causing it to look for a test named "test"
# The catch-all rule prevents make from trying to execute pytest args as make targets
%:
	@: