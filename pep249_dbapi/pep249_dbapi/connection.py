"""
PEP 249 Database API 2.0 Connection Objects

This module defines the Connection class as specified in PEP 249.
"""
from .api_client.client_api import client_api_init
from .api_client.c_api import CORE_API
from .cursor import Cursor
from .exceptions import NotSupportedError, InterfaceError
from .thrift_gen.database_driver_v1 import DatabaseDriver
from .thrift_gen.database_driver_v1.ttypes import TlsConfig, CertRevocationCheckMode


class Connection:
    """
    Connection objects represent a database connection.
    """
    
    def __init__(self, **kwargs):
        """
        Initialize a new connection object.
        
        Args:
            database: Database name
            user: Username
            password: Password
            host: Host name
            port: Port number
            **kwargs: Additional connection parameters
        """
        self.db_api: DatabaseDriver.Client  = client_api_init(CORE_API.DATABASE_DRIVER_API_V1)
        self.db_handle = self.db_api.databaseNew()
        self.db_api.databaseInit(self.db_handle)
        self.conn_handle = self.db_api.connectionNew()

        # Optional TLS config
        tls_config = TlsConfig()
        tls_configured = False

        for key, value in kwargs.items():
            # First, capture TLS-related kwargs into TlsConfig; don't pass them as generic options
            if key == "cert_revocation_check_mode" and isinstance(value, str):
                mode = value.upper()
                if mode == "ENABLED":
                    tls_config.crl_mode = CertRevocationCheckMode.ENABLED
                elif mode == "ADVISORY":
                    tls_config.crl_mode = CertRevocationCheckMode.ADVISORY
                else:
                    tls_config.crl_mode = CertRevocationCheckMode.DISABLED
                tls_configured = True
                continue
            if key == "enable_crl_disk_caching" and isinstance(value, str):
                tls_config.crl_disk_caching = (value.lower() == "true")
                tls_configured = True
                continue
            if key == "enable_crl_memory_caching" and isinstance(value, str):
                tls_config.crl_memory_caching = (value.lower() == "true")
                tls_configured = True
                continue
            if key == "sf_crl_response_cache_dir" and isinstance(value, str):
                tls_config.crl_cache_dir = value
                tls_configured = True
                continue
            if key == "sf_crl_validity_time" and isinstance(value, int):
                tls_config.crl_validity_days = value
                tls_configured = True
                continue
            if key == "allow_certificates_without_crl_url" and isinstance(value, str):
                tls_config.allow_certs_without_crl_url = (value.lower() == "true")
                tls_configured = True
                continue
            if key == "crl_http_timeout" and isinstance(value, int):
                tls_config.crl_http_timeout_seconds = value
                tls_configured = True
                continue
            if key == "crl_connection_timeout" and isinstance(value, int):
                tls_config.crl_connection_timeout_seconds = value
                tls_configured = True
                continue
            if key == "custom_root_store_path" and isinstance(value, str):
                tls_config.custom_root_store_path = value
                tls_configured = True
                continue
            if key == "verify_hostname" and isinstance(value, str):
                tls_config.verify_hostname = (value.lower() == "true")
                tls_configured = True
                continue
            if key == "verify_certificates" and isinstance(value, str):
                tls_config.verify_certificates = (value.lower() == "true")
                tls_configured = True
                continue

            # Snowflake-style aliases
            if key == "insecure_mode" and isinstance(value, str):
                insecure = (value.lower() == "true")
                tls_config.verify_hostname = (not insecure)
                tls_config.verify_certificates = (not insecure)
                tls_configured = True
                continue
            if key == "ocsp_fail_open" and isinstance(value, str):
                fail_open = (value.lower() == "true")
                tls_config.crl_mode = CertRevocationCheckMode.ADVISORY if fail_open else CertRevocationCheckMode.ENABLED
                tls_configured = True
                continue

            # Non-TLS options go through the generic setters
            if isinstance(value, int):
                self.db_api.connectionSetOptionInt(self.conn_handle, key, value)

            if isinstance(value, str):
                self.db_api.connectionSetOptionString(self.conn_handle, key, value)

            if isinstance(value, float):
                self.db_api.connectionSetOptionDouble(self.conn_handle, key, value)

        if tls_configured:
            self.db_api.connectionSetTlsConfig(self.conn_handle, tls_config)
        self.db_api.connectionInit(self.conn_handle, self.db_handle)
        self.kwargs = kwargs
        self._closed = False
        self._autocommit = False

    def close(self):
        """
        Close the connection now.
        """
        self._closed = True

    def commit(self):
        """
        Commit any pending transaction to the database.

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("commit is not implemented")

    def rollback(self):
        """
        Roll back to the start of any pending transaction.

        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("rollback is not implemented")

    def cursor(self):
        """
        Return a new Cursor object using the connection.

        Returns:
            Cursor: A new cursor object
        """
        if self._closed:
            raise InterfaceError("Connection is closed")
        return Cursor(self)

    # Context manager support
    def __enter__(self):
        """
        Enter the runtime context for the connection.

        Returns:
            Connection: Self
        """
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        """
        Exit the runtime context for the connection.
        
        If an exception occurred, rollback the transaction.
        Otherwise, commit the transaction.
        """
        if exc_type is None:
            # No exception, commit
            try:
                self.commit()
            except NotSupportedError:
                pass  # commit not implemented
        else:
            # Exception occurred, rollback
            try:
                self.rollback()
            except NotSupportedError:
                pass  # rollback not implemented
        
        self.close()
    
    # Optional methods that some databases might support
    def cancel(self):
        """
        Cancel a long-running operation on the connection.
        
        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("cancel is not implemented")
    
    def ping(self):
        """
        Check if the connection to the server is still alive.
        
        Returns:
            bool: True if connection is alive, False otherwise
            
        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("ping is not implemented")
    
    def set_autocommit(self, autocommit):
        """
        Set the autocommit mode.
        
        Args:
            autocommit (bool): True to enable autocommit, False to disable
            
        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("set_autocommit is not implemented")
    
    def get_autocommit(self):
        """
        Get the current autocommit mode.
        
        Returns:
            bool: Current autocommit setting
            
        Raises:
            NotSupportedError: If not implemented
        """
        raise NotSupportedError("get_autocommit is not implemented")
    
    @property
    def autocommit(self):
        """
        Get/set autocommit mode as a property.
        
        Returns:
            bool: Current autocommit setting
        """
        return self._autocommit
    
    @autocommit.setter
    def autocommit(self, value):
        """
        Set autocommit mode.
        
        Args:
            value (bool): Autocommit setting
        """
        self._autocommit = value
        try:
            self.set_autocommit(value)
        except NotSupportedError:
            pass  # autocommit not supported by implementation

    def is_closed(self):
        """
        Check if the connection is closed.

        Returns:
            bool: True if connection is closed, False otherwise
        """
        return self._closed
