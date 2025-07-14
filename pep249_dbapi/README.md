# PEP 249 Database API 2.0 Implementation

A Python library that implements [PEP 249 (Python Database API Specification 2.0)](https://peps.python.org/pep-0249/) with empty interface implementations. This library provides a complete skeleton implementation that follows the PEP 249 specification, making it an ideal starting point for creating new database drivers or for testing database API compliance.

## Features

- **Complete PEP 249 compliance**: Implements all required interfaces, constants, and exception hierarchy
- **Empty implementation**: All methods raise `NotSupportedError` by default, allowing incremental implementation
- **Full type support**: Includes all required type constructors and type objects
- **Context manager support**: Both Connection and Cursor objects support the `with` statement
- **Iterator protocol**: Cursor objects are iterable
- **Python 2/3 compatibility**: Includes compatibility features for both Python versions
- **Comprehensive tests**: Full test suite using pytest

## Installation

```bash
pip install pep249-dbapi
```

For development:

```bash
pip install pep249-dbapi[dev]
```

## Quick Start

```python
import pep249_dbapi

# Module-level constants
print(f"API Level: {pep249_dbapi.apilevel}")
print(f"Thread Safety: {pep249_dbapi.threadsafety}")
print(f"Parameter Style: {pep249_dbapi.paramstyle}")

# Create a connection
conn = pep249_dbapi.connect(
    database="mydb",
    user="myuser",
    password="mypass",
    host="localhost"
)

# Use connection as context manager
with conn:
    # Create a cursor
    cursor = conn.cursor()
    
    # Use cursor as context manager
    with cursor:
        # Execute a query (will raise NotSupportedError in base implementation)
        try:
            cursor.execute("SELECT * FROM users")
            results = cursor.fetchall()
        except pep249_dbapi.NotSupportedError:
            print("Method not implemented yet")
```

## API Reference

### Module Constants

- `apilevel`: String constant stating the supported DB API level ("2.0")
- `threadsafety`: Integer constant stating the level of thread safety (1)
- `paramstyle`: String constant stating the type of parameter marker formatting ("format")

### Functions

- `connect(**kwargs)`: Create a new database connection

### Exception Hierarchy

```
Warning
Error
├── InterfaceError
└── DatabaseError
    ├── DataError
    ├── OperationalError
    ├── IntegrityError
    ├── InternalError
    ├── ProgrammingError
    └── NotSupportedError
```

### Type Constructors

- `Date(year, month, day)`: Construct a date object
- `Time(hour, minute, second)`: Construct a time object
- `Timestamp(year, month, day, hour, minute, second)`: Construct a timestamp object
- `DateFromTicks(ticks)`: Construct a date from seconds since epoch
- `TimeFromTicks(ticks)`: Construct a time from seconds since epoch
- `TimestampFromTicks(ticks)`: Construct a timestamp from seconds since epoch
- `Binary(data)`: Construct a binary object

### Type Objects

- `STRING`: Type object for string-like columns
- `BINARY`: Type object for binary columns
- `NUMBER`: Type object for numeric columns
- `DATETIME`: Type object for date/time columns
- `ROWID`: Type object for row ID columns

### Connection Objects

#### Methods

- `close()`: Close the connection
- `commit()`: Commit pending transactions (raises NotSupportedError)
- `rollback()`: Rollback pending transactions (raises NotSupportedError)
- `cursor()`: Create a new cursor object

#### Properties

- `autocommit`: Get/set autocommit mode

### Cursor Objects

#### Attributes

- `description`: Describes result columns (read-only)
- `rowcount`: Number of rows affected by last operation (read-only)
- `arraysize`: Number of rows to fetch at a time (default: 1)

#### Methods

- `callproc(procname, parameters=None)`: Call a stored procedure (raises NotSupportedError)
- `close()`: Close the cursor
- `execute(operation, parameters=None)`: Execute an operation (raises NotSupportedError)
- `executemany(operation, seq_of_parameters)`: Execute operation multiple times (raises NotSupportedError)
- `fetchone()`: Fetch next row (raises NotSupportedError)
- `fetchmany(size=None)`: Fetch multiple rows (raises NotSupportedError)
- `fetchall()`: Fetch all remaining rows (raises NotSupportedError)
- `nextset()`: Move to next result set (raises NotSupportedError)
- `setinputsizes(sizes)`: Set input sizes (no-op)
- `setoutputsize(size, column=None)`: Set output size (no-op)

## Extending the Implementation

To create a working database driver, inherit from the provided classes and implement the required methods:

```python
from pep249_dbapi import Connection as BaseConnection, Cursor as BaseCursor

class MyConnection(BaseConnection):
    def commit(self):
        # Implement actual commit logic
        pass
    
    def rollback(self):
        # Implement actual rollback logic
        pass

class MyCursor(BaseCursor):
    def execute(self, operation, parameters=None):
        # Implement actual query execution
        pass
    
    def fetchone(self):
        # Implement actual row fetching
        pass
```

## Testing

Run the test suite:

```bash
pytest
```

Run with coverage:

```bash
pytest --cov=pep249_dbapi --cov-report=html
```

## Development

Install in development mode:

```bash
pip install -e .[dev]
```

Run code formatting:

```bash
black pep249_dbapi tests
```

Run linting:

```bash
flake8 pep249_dbapi tests
```

Run type checking:

```bash
mypy pep249_dbapi
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## References

- [PEP 249 - Python Database API Specification v2.0](https://peps.python.org/pep-0249/)
- [Python Database API Specification v2.0](https://www.python.org/dev/peps/pep-0249/) 