#!/usr/bin/env python

import sys
from config import config


def install_compatibility_layer():
    
    config.setup_environment()
    import pep249_dbapi
    
    
    class CursorModule:
        
        def __getattr__(self, name):
            if name == 'SnowflakeCursor':
                return pep249_dbapi.Cursor
            else:
                try:
                    return getattr(pep249_dbapi.cursor, name)
                except AttributeError:
                    raise AttributeError(f"module 'snowflake.connector.cursor' has no attribute '{name}'")
    
    class ErrorsModule:
        
        def __getattr__(self, name):
            try:
                return getattr(pep249_dbapi.exceptions, name)
            except AttributeError:
                try:
                    return getattr(pep249_dbapi, name)
                except AttributeError:
                    raise AttributeError(f"module 'snowflake.connector.errors' has no attribute '{name}'")
    
    cursor_module = CursorModule()
    errors_module = ErrorsModule()
    
    class UtilTextModule:
        def random_string(self, length=10, prefix=""):
            import random
            import string
            random_part = ''.join(random.choices(string.ascii_letters + string.digits, k=length))
            return prefix + random_part
    
    util_text_module = UtilTextModule()
    
    class CompatModule:
        import platform
        IS_LINUX = platform.system() == "Linux"
        IS_WINDOWS = platform.system() == "Windows"
        IS_MACOS = platform.system() == "Darwin"
    
    compat_module = CompatModule()
    
    class TelemetryModule:
        class TelemetryClient:
            def __init__(self, *args, **kwargs):
                pass
            def send(self, *args, **kwargs):
                pass
        
        class TelemetryData:
            def __init__(self, *args, **kwargs):
                pass
    
    telemetry_module = TelemetryModule()
    
    class TelemetryOOBModule:
        class TelemetryService:
            def __init__(self, *args, **kwargs):
                pass
            def send(self, *args, **kwargs):
                pass
    
    telemetry_oob_module = TelemetryOOBModule()
    
    class ConnectionModule:
        Connection = pep249_dbapi.Connection
        
        class DefaultConverterClass:
            def __init__(self, *args, **kwargs):
                pass
        
        def __getattr__(self, name):
            try:
                return getattr(pep249_dbapi.connection, name)
            except AttributeError:
                raise AttributeError(f"module 'snowflake.connector.connection' has no attribute '{name}'")
    
    connection_module = ConnectionModule()
    
    import requests as std_requests
    import urllib3 as std_urllib3
    
    class VendoredModule:
        requests = std_requests
        urllib3 = std_urllib3
    
    vendored_module = VendoredModule()
    
    sys.modules['snowflake.connector.vendored.requests'] = std_requests
    sys.modules['snowflake.connector.vendored.urllib3'] = std_urllib3
    
    import logging
    
    class SecretDetectorModule:
        class SecretDetector(logging.Formatter):
            def format(self, record: logging.LogRecord) -> str:
                return super().format(record)
    
    secret_detector_module = SecretDetectorModule()
    
    
    class MinimalSnowflakeConnector:
        SnowflakeConnection = pep249_dbapi.Connection
        
        def connect(self, **kwargs):
            
            import json
            from pathlib import Path
            
            params_file = config.universal_driver_root / "parameters.json"
            if params_file.exists():
                with open(params_file, 'r') as f:
                    params_data = json.load(f)
                    test_params = params_data.get('testconnection', {})
                
                kwargs['user'] = test_params.get('SNOWFLAKE_TEST_USER', kwargs.get('user'))
                kwargs['password'] = test_params.get('SNOWFLAKE_TEST_PASSWORD', kwargs.get('password'))
                kwargs['account'] = test_params.get('SNOWFLAKE_TEST_ACCOUNT', kwargs.get('account'))
                kwargs['host'] = test_params.get('SNOWFLAKE_TEST_HOST', kwargs.get('host'))
                kwargs['database'] = test_params.get('SNOWFLAKE_TEST_DATABASE', kwargs.get('database'))
                kwargs['schema'] = test_params.get('SNOWFLAKE_TEST_SCHEMA', kwargs.get('schema'))
                kwargs['warehouse'] = test_params.get('SNOWFLAKE_TEST_WAREHOUSE', kwargs.get('warehouse'))
                kwargs['role'] = test_params.get('SNOWFLAKE_TEST_ROLE', kwargs.get('role'))
                kwargs['authenticator'] = 'SNOWFLAKE_PASSWORD'
            
            if 'private_key_file' in kwargs and kwargs['private_key_file'] == '<private_key_file>':
                del kwargs['private_key_file']
            
            return pep249_dbapi.connect(**kwargs)
        
        def __getattr__(self, name):
            return getattr(pep249_dbapi, name)
    
    snowflake_connector = MinimalSnowflakeConnector()
    snowflake_module = type('SnowflakeModule', (), {})()
    snowflake_module.connector = snowflake_connector
    sys.modules['snowflake'] = snowflake_module
    sys.modules['snowflake.connector'] = snowflake_connector
    sys.modules['snowflake.connector.cursor'] = cursor_module
    sys.modules['snowflake.connector.errors'] = errors_module
    sys.modules['snowflake.connector.util_text'] = util_text_module
    sys.modules['snowflake.connector.compat'] = compat_module
    sys.modules['snowflake.connector.telemetry'] = telemetry_module
    sys.modules['snowflake.connector.telemetry_oob'] = telemetry_oob_module
    sys.modules['snowflake.connector.connection'] = connection_module
    sys.modules['snowflake.connector.vendored'] = vendored_module
    sys.modules['snowflake.connector.secret_detector'] = secret_detector_module
    


if __name__ == "__main__":
    install_compatibility_layer()
    
    try:
        from snowflake.connector import connect
        try:
            from snowflake.connector import OperationalError
        except AttributeError:
            pass
    except Exception:
        sys.exit(1)