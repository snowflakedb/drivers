# -------- Settings (override at call) --------
PY             ?= $(shell python3 -c "import sys; print(f'{sys.version_info.major}.{sys.version_info.minor}')")
PYID           := $(subst .,,$(PY))
REPORTS_DIR    ?= reports
REF_VERSION    ?= 3.10.1
FAIL_ON_REGRESSIONS ?= 0              # should the pipeline fail if there is discrepancy between original driver and the new (universal) implementation
POSARGS        ?=                     # extra pytest args, e.g. -k smoke -m reference

SHELL := bash
.ONESHELL:

.PHONY: install-tools test-local test-unit-local test-integ-local unit-universal integ-universal integ-reference compare compare-artifacts compare-local clean-reports

install-tools:
	python3 -m pip install -U pip pipx
	pipx install tox
	pipx inject tox tox-uv
	mkdir -p $(REPORTS_DIR)

# Local testing - replaces run_tests.sh
test-local:
	cd .. && cargo build --package sf_core
	$(eval CORE_PATH := $(shell cd .. && pwd)/target/debug/$(shell uname -s | sed 's/Darwin/libsf_core.dylib/; s/Linux/libsf_core.so/'))
	$(eval PARAMETER_PATH := $(shell cd .. && pwd)/parameters.json)
	python3 -m pip install -e ".[dev]"
	CORE_PATH=$(CORE_PATH) PARAMETER_PATH=$(PARAMETER_PATH) pytest tests/ -v $(POSARGS)

test-unit-local:
	cd .. && cargo build --package sf_core
	$(eval CORE_PATH := $(shell cd .. && pwd)/target/debug/$(shell uname -s | sed 's/Darwin/libsf_core.dylib/; s/Linux/libsf_core.so/'))
	python3 -m pip install -e ".[dev]"
	CORE_PATH=$(CORE_PATH) pytest tests/unit/ -v $(POSARGS)

test-integ-local:
	cd .. && cargo build --package sf_core
	$(eval CORE_PATH := $(shell cd .. && pwd)/target/debug/$(shell uname -s | sed 's/Darwin/libsf_core.dylib/; s/Linux/libsf_core.so/'))
	$(eval PARAMETER_PATH := $(shell cd .. && pwd)/parameters.json)
	python3 -m pip install -e ".[dev]"
	CORE_PATH=$(CORE_PATH) PARAMETER_PATH=$(PARAMETER_PATH) pytest tests/integ/ --connector=universal -v $(POSARGS)

unit-universal:
	mkdir -p $(REPORTS_DIR)
	python3 -m pip install -e ".[dev]"
	pytest tests/unit/ -v --tb=short \
	  --junitxml=$(REPORTS_DIR)/unit-$(PY).xml

integ-universal: install-tools
	mkdir -p $(REPORTS_DIR)
	cd .. && tox -e py$(PYID)-integ-universal -- \
	  pep249_dbapi/tests/integ \
	  $(POSARGS) \
	  --json-report --json-report-file=pep249_dbapi/$(REPORTS_DIR)/universal-$(PY).json \
	  --junitxml=pep249_dbapi/$(REPORTS_DIR)/universal-$(PY).xml

integ-reference: install-tools
	mkdir -p $(REPORTS_DIR)
	cd .. && tox -e py$(PYID)-integ-reference -- \
	  pep249_dbapi/tests/integ \
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
compare-local: integ-universal integ-reference compare

# Use this when comparing downloaded CI artifacts
compare-artifacts:
	python ci/reference_tests/compare_reports.py \
	  --py $(PY) \
	  --universal $${UNIVERSAL_DIR}/universal-$(PY).json \
	  --reference $${REFERENCE_DIR}/reference-$(PY).json \
	  --fail-on-regressions $(FAIL_ON_REGRESSIONS)

clean-reports:
	rm -rf $(REPORTS_DIR)
