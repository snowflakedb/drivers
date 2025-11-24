"""
PEP 249 Database API 2.0 Connection Objects

This module defines the Connection class as specified in PEP 249.
"""
from ._internal.api_client.client_api import database_driver_client
from .cursor import Cursor
from .exceptions import NotSupportedError, InterfaceError


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
        self.db_api = database_driver_client()
        self.db_handle = self.db_api.database_new(DatabaseNewRequest()).db_handle
        self.db_api.database_init(DatabaseInitRequest(db_handle=self.db_handle))
        self.conn_handle = self.db_api.connection_new(ConnectionNewRequest()).conn_handle
        for key, value in kwargs.items():
            if isinstance(value, int):
                self.db_api.connection_set_option_int(ConnectionSetOptionIntRequest(conn_handle=self.conn_handle, key=key, value=value))

            if isinstance(value, str):
                self.db_api.connection_set_option_string(ConnectionSetOptionStringRequest(conn_handle=self.conn_handle, key=key, value=value))

            if isinstance(value, float):
                self.db_api.connection_set_option_double(ConnectionSetOptionDoubleRequest(conn_handle=self.conn_handle, key=key, value=value))

        self.db_api.connection_init(ConnectionInitRequest(conn_handle=self.conn_handle, db_handle=self.db_handle))
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
