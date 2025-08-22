# -------- Settings (override at call) --------
PY             ?= $(shell python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
PYID           := $(subst .,,$(PY))
REPORTS_DIR    ?= reports
REF_VERSION    ?= 3.10.1
FAIL_ON_REGRESSIONS ?= 0              # should the pipeline fail if there is discrepancy between original driver and the new (universal) implementation
POSARGS        ?=                     # extra pytest args, e.g. -k smoke -m reference

# Required environment variables (no defaults)
# CORE_PATH      - Path to the Rust core library
# PARAMETER_PATH - Path to test parameters JSON file

SHELL := bash
.ONESHELL:

.PHONY: check-env-or-build check-env-or-fail install-tools test-local test-unit-local test-integ-local test-reference-local unit unit-universal all-universal integ-reference compare compare-artifacts test-and-compare-local clean-reports

# For local development - build core if missing
check-env-or-build:
	@if [ -z "$(CORE_PATH)" ]; then \
		echo "Error: CORE_PATH environment variable not set"; \
		echo "Set it to the path of your Rust core library"; \
		echo "Example: export CORE_PATH=../target/debug/libsf_core.dylib"; \
		exit 1; \
	fi
	@if [ ! -f "$(CORE_PATH)" ]; then \
		echo "Core library not found, building..."; \
		cd .. && cargo build --package sf_core || { \
			echo "Failed to build Rust core library"; \
			echo "Make sure you're in a virtual environment: source venv/bin/activate"; \
			exit 1; \
		}; \
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

install-tools:
	@python3 -m pip install -U pip tox tox-uv pytest-xdist pytest-json-report || { \
		echo "Failed to install Python tools"; \
		echo "Make sure you're in a virtual environment: source venv/bin/activate"; \
		exit 1; \
	}
	mkdir -p $(REPORTS_DIR)

# ===== Local Development Commands =====

# Local testing - replaces run_tests.sh  
test-local: check-env-or-build
	@python3 -m pip install -e ".[dev]" || { \
		echo "Failed to install package"; \
		echo "Make sure you're in a virtual environment: source venv/bin/activate"; \
		exit 1; \
	}
	CORE_PATH=$(CORE_PATH) PARAMETER_PATH=$(PARAMETER_PATH) pytest tests/ -v $(POSARGS)

test-unit-local: check-env-or-build
	@python3 -m pip install -e ".[dev]" || { \
		echo "Failed to install package"; \
		echo "Make sure you're in a virtual environment: source venv/bin/activate"; \
		exit 1; \
	}
	CORE_PATH=$(CORE_PATH) pytest tests/unit/ -v $(POSARGS)

test-integ-local: check-env-or-build
	@python3 -m pip install -e ".[dev]" || { \
		echo "Failed to install package"; \
		echo "Make sure you're in a virtual environment: source venv/bin/activate"; \
		exit 1; \
	}
	mkdir -p $(REPORTS_DIR)
	CORE_PATH=$(CORE_PATH) PARAMETER_PATH=$(PARAMETER_PATH) pytest tests/integ/ --connector=universal -v \
	  --json-report --json-report-file=$(REPORTS_DIR)/universal-$(PY).json $(POSARGS)

test-reference-local: check-env-or-build
	@python3 -m pip install -e ".[dev]" || { \
		echo "Failed to install package"; \
		echo "Make sure you're in a virtual environment: source venv/bin/activate"; \
		exit 1; \
	}
	mkdir -p $(REPORTS_DIR)
	CORE_PATH=$(CORE_PATH) PARAMETER_PATH=$(PARAMETER_PATH) pytest tests/integ/ --connector=reference -v \
	  --json-report --json-report-file=$(REPORTS_DIR)/reference-$(PY).json $(POSARGS)

# ===== CI/Tox Commands =====

unit: check-env-or-fail install-tools
	mkdir -p $(REPORTS_DIR)
	cd .. && CORE_PATH=$(CORE_PATH) tox -e py$(PYID)-unit -- \
	  $(POSARGS) \
	  --junitxml=pep249_dbapi/$(REPORTS_DIR)/unit-$(PY).xml

unit-universal: check-env-or-fail
	mkdir -p $(REPORTS_DIR)
	@python3 -m pip install -e ".[dev]" || { \
		echo "Failed to install package"; \
		echo "Make sure you're in a virtual environment: source venv/bin/activate"; \
		exit 1; \
	}
	CORE_PATH=$(CORE_PATH) pytest tests/unit/ -v --tb=short \
	  --junitxml=$(REPORTS_DIR)/unit-$(PY).xml

all-universal: check-env-or-fail install-tools
	mkdir -p $(REPORTS_DIR)
	cd .. && CORE_PATH=$(CORE_PATH) PARAMETER_PATH=$(PARAMETER_PATH) tox -e py$(PYID)-all-universal -- \
	  $(POSARGS) \
	  --json-report --json-report-file=pep249_dbapi/$(REPORTS_DIR)/universal-all-$(PY).json \
	  --junitxml=pep249_dbapi/$(REPORTS_DIR)/universal-all-$(PY).xml
	python ci/extract_integ_results.py \
	  --input $(REPORTS_DIR)/universal-all-$(PY).json \
	  --output $(REPORTS_DIR)/universal-$(PY).json

integ-reference: check-env-or-fail install-tools
	mkdir -p $(REPORTS_DIR)
	cd .. && CORE_PATH=$(CORE_PATH) PARAMETER_PATH=$(PARAMETER_PATH) tox -e py$(PYID)-integ-reference -- \
	  $(POSARGS) \
	  --json-report --json-report-file=pep249_dbapi/$(REPORTS_DIR)/reference-$(PY).json \
	  --junitxml=pep249_dbapi/$(REPORTS_DIR)/reference-$(PY).xml

compare:
	python ci/reference_tests/compare_reports.py \
	  --py $(PY) \
	  --universal $(REPORTS_DIR)/universal-$(PY).json \
	  --reference $(REPORTS_DIR)/reference-$(PY).json \
	  --fail-on-regressions $(FAIL_ON_REGRESSIONS)

# One-shot local run: universal → reference → diff
test-and-compare-local: test-integ-local test-reference-local compare

# Use this when comparing downloaded CI artifacts
compare-artifacts:
	python ci/reference_tests/compare_reports.py \
	  --py $(PY) \
	  --universal $${UNIVERSAL_DIR}/universal-$(PY).json \
	  --reference $${REFERENCE_DIR}/reference-$(PY).json \
	  --fail-on-regressions $(FAIL_ON_REGRESSIONS)

clean-reports:
	rm -rf $(REPORTS_DIR)
