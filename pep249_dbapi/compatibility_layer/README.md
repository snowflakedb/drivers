# Snowflake Test Compatibility Runner

**Production-ready solution** for running old `snowflake-connector-python` tests against the new universal driver with **pure compatibility testing** - no mocking, real failures expose real incompatibilities.

## 🚀 Quick Start

```bash
# 1. Setup (validates environment + installs dependencies)
python setup.py

# 2. Run any old test with full path
python runner.py /path/to/snowflake-connector-python/test/unit/test_file.py::test_method -v
```

## 🎯 How It Works

**Simple redirect**: `snowflake.connector` imports → `pep249_dbapi`

- ✅ **No complex wrappers or mocking**
- ✅ **Real failures** when APIs don't match  
- ✅ **Automatic file copying** to avoid conftest.py conflicts
- ✅ **Auto-detection** of all paths (no hardcoding)

## 📋 Usage

```bash
# Run specific test method
python runner.py /path/to/test_errors.py::test_args -v

# Run entire file  
python runner.py /path/to/test_file.py -v

# Run with pytest options
python runner.py /path/to/test_file.py -k "not slow" --tb=short

# Validate environment only
python runner.py --validate-only /dev/null
```

## 📁 Production Architecture

```
old_tests_with_new_driver/
├── setup.py              # One-command setup & validation
├── runner.py              # Universal test runner with auto-copying  
├── config.py              # Auto-detection (no hardcoded paths)
├── compatibility.py       # Simple interface
├── real_snowflake_compatibility_layer.py  # Direct redirect (no wrappers)
└── requirements.txt       # Dependencies
```

## ✅ Expected Results

### **PEP 249 Compatibility: ~100% Success**
Tests using standard DB-API functionality work perfectly:
- Basic imports: `connect`, `OperationalError`, `Error`
- Standard exception creation: `OperationalError("message")`
- Module attributes: `apilevel`, `threadsafety`, `paramstyle`

### **Proprietary Extensions: ~95% Failures**  
Tests using old driver's Snowflake-specific extensions fail with real errors:
- `ImportError: cannot import name 'errors' from 'pep249_dbapi'`
- `ModuleNotFoundError: No module named 'snowflake.connector.compat'`
- `TypeError: ProgrammingError() takes no keyword arguments`

**These failures are valuable** - they show exactly what proprietary features would need to be implemented for full backward compatibility.

## 🔧 Key Features

- ✅ **Pure compatibility testing** - exposes real API differences
- ✅ **Automatic file copying** - seamless path handling
- ✅ **No hardcoded paths** - auto-detects all components
- ✅ **Production ready** - proper error handling and validation
- ✅ **Simple architecture** - direct redirect, minimal complexity

## 🎯 Philosophy

**Expose real incompatibilities, don't hide them.**

The solution provides genuine compatibility assessment by showing:
- What works (standard PEP 249 DB-API)
- What fails (proprietary Snowflake extensions)

This gives clear guidance on what needs to be implemented in the new driver for full backward compatibility.

## 🚨 Requirements

- Universal driver built: `cd universal-driver && cargo build`
- Python 3.8+ with pytest
- Auto-detects: universal driver, core library, old driver paths