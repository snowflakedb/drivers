# -------- Settings (override at call) --------
PYTHON_VERSION ?= $(shell python3 -c 'import sys;print(f"{sys.version_info.major}.{sys.version_info.minor}")')
PYTHON_ID := $(subst .,,$(PYTHON_VERSION))
REPORTS_DIR ?= reports
REFERENCE_DRIVER_VERSION ?= 3.17.2
PYTEST_ARGS ?=
FAIL_ON_REGRESSIONS ?= 0

# Default paths for local development (can be overridden)
DEFAULT_CORE_PATH_MACOS := $(shell cd .. && pwd)/target/debug/libsf_core.dylib
DEFAULT_CORE_PATH_LINUX := $(shell cd .. && pwd)/target/debug/libsf_core.so
DEFAULT_PARAMETER_PATH := $(shell cd .. && pwd)/parameters.json

SHELL := bash
.ONESHELL:

.PHONY: check-env-or-build check-env-or-fail setup test test-local tox test-local-tox test-unit-local test-integ-local test-reference-local test-reference-local-tox test-integ-universal test-integ-reference compare compare-artifacts compare-local clean-reports

# Auto-detect platform-specific paths
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

# For local development - build Rust core if missing
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

# For CI - fail if core missing
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

# ===== Local Development Commands =====

# Run all tests (unit + integration) with direct pytest
test-local: check-env-or-build setup
	CORE_PATH=$(DETECTED_CORE_PATH) PARAMETER_PATH=$(DETECTED_PARAMETER_PATH) uv run pytest tests/ -n auto -v $(PYTEST_ARGS) $(filter-out test-local test,$(MAKECMDGOALS))

# Alias for test-local
test: test-local

# Run integration tests with tox + xdist for faster parallel execution
test-local-tox: check-env-or-build setup
	CORE_PATH=$(DETECTED_CORE_PATH) PARAMETER_PATH=$(DETECTED_PARAMETER_PATH) uv run tox -e py$(PYTHON_ID)-integ-universal -- \
	  --json-report --json-report-file=$(REPORTS_DIR)/universal-$(PYTHON_VERSION).json \
	  -n auto $(PYTEST_ARGS) $(filter-out test-local-tox tox,$(MAKECMDGOALS))

# Alias for test-local-tox
tox: test-local-tox

# Run only unit tests with direct pytest
test-unit-local: check-env-or-build setup
	CORE_PATH=$(DETECTED_CORE_PATH) uv run pytest tests/unit/ -v $(PYTEST_ARGS) $(filter-out test-unit-local,$(MAKECMDGOALS))

# Run integration tests with universal connector (direct pytest with xdist)
test-integ-local: check-env-or-build setup
	CORE_PATH=$(DETECTED_CORE_PATH) PARAMETER_PATH=$(DETECTED_PARAMETER_PATH) uv run pytest tests/integ/ --connector=universal -v -n auto \
	  --json-report --json-report-file=$(REPORTS_DIR)/universal-$(PYTHON_VERSION).json $(PYTEST_ARGS) $(filter-out test-integ-local,$(MAKECMDGOALS))

# Run integration tests with reference connector (direct pytest with xdist)
test-reference-local: check-env-or-build setup
	CORE_PATH=$(DETECTED_CORE_PATH) PARAMETER_PATH=$(DETECTED_PARAMETER_PATH) uv run pytest tests/integ/ --connector=reference -v -n auto \
	  --json-report --json-report-file=$(REPORTS_DIR)/reference-$(PYTHON_VERSION).json $(PYTEST_ARGS) $(filter-out test-reference-local,$(MAKECMDGOALS))

# Run integration tests with reference connector using tox + xdist for faster parallel execution
test-reference-local-tox: check-env-or-build setup
	CORE_PATH=$(DETECTED_CORE_PATH) PARAMETER_PATH=$(DETECTED_PARAMETER_PATH) uv run tox -e py$(PYTHON_ID)-integ-reference -- \
	  --json-report --json-report-file=$(REPORTS_DIR)/reference-$(PYTHON_VERSION).json \
	  -n auto $(PYTEST_ARGS)

# ===== CI/Tox Commands =====

# CI: Run integration tests with universal connector (expects CORE_PATH set, uv pre-installed)
test-integ-universal: check-env-or-fail
	mkdir -p $(REPORTS_DIR)
	uv run tox -e py$(PYTHON_ID)-integ-universal -- \
	  --json-report --json-report-file=$(REPORTS_DIR)/universal-$(PYTHON_VERSION).json \
	  --junitxml=$(REPORTS_DIR)/universal-$(PYTHON_VERSION).xml \
	  $(PYTEST_ARGS)

# CI: Run integration tests with reference connector (expects CORE_PATH set, uv pre-installed)
test-integ-reference: check-env-or-fail
	mkdir -p $(REPORTS_DIR)
	uv run tox -e py$(PYTHON_ID)-integ-reference -- \
	  --json-report --json-report-file=$(REPORTS_DIR)/reference-$(PYTHON_VERSION).json \
	  --junitxml=$(REPORTS_DIR)/reference-$(PYTHON_VERSION).xml \
	  $(PYTEST_ARGS)

# ===== Comparison Commands =====

compare:
	python3 ci/reference_tests/compare_reports.py \
	  --py $(PYTHON_VERSION) \
	  --universal $(REPORTS_DIR)/universal-$(PYTHON_VERSION).json \
	  --reference $(REPORTS_DIR)/reference-$(PYTHON_VERSION).json \
	  --fail-on-regressions $(FAIL_ON_REGRESSIONS)

# Fast comparison using tox for parallel execution in isolated env
compare-local:
	$(MAKE) test-local-tox || true
	$(MAKE) test-reference-local-tox || true   # tests can fail and still we want the final report
	$(MAKE) compare

# Use this when comparing downloaded CI artifacts
compare-artifacts:
	python3 ci/reference_tests/compare_reports.py \
	  --py $(PYTHON_VERSION) \
	  --universal $(UNIVERSAL_TEST_REPORTS_DIR)/universal-$(PYTHON_VERSION).json \
	  --reference $(REFERENCE_TEST_REPORTS_DIR)/reference-$(PYTHON_VERSION).json \
	  --fail-on-regressions $(FAIL_ON_REGRESSIONS)

# ===== Utility Commands =====

clean-reports:
	rm -rf $(REPORTS_DIR)

# Catch-all rule to prevent make from trying to execute pytest args as targets
%:
	@: