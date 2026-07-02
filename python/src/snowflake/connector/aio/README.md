# Asynchronous Snowflake Python Connector (aio)

The Snowflake Python connector offers an asyncio-compatible interface via the
`snowflake.connector.aio` module. Using the async version enables a single
thread to handle hundreds of concurrent database connections without blocking,
improving throughput and reducing resource usage for I/O-bound workloads like
multiple parallel queries.

---

## Installation & Import

```python
# Same package, different import path
from snowflake.connector.aio import connect, SnowflakeConnection, DictCursor
```

---

## Connection Patterns

```python
import asyncio
from snowflake.connector.aio import connect, SnowflakeConnection

# Pattern 1: Async context manager (recommended)
async with connect(user='...', password='...', account='...') as conn:
    pass

# Pattern 2: Direct await
conn = await connect(user='...', password='...', account='...')
await conn.close()

# Pattern 3: Manual instantiation
conn = SnowflakeConnection(user='...', password='...', account='...')
await conn.connect()
await conn.close()
```

---

## Basic Operations Comparison

| Operation | Sync | Async |
|-----------|------|-------|
| **Connect** | `conn = connect(...)` | `conn = await connect(...)` |
| **Create cursor** | `cur = conn.cursor()` | `cur = conn.cursor()` *(same)* |
| **Execute** | `cur.execute(sql)` | `await cur.execute(sql)` |
| **Fetch one** | `cur.fetchone()` | `await cur.fetchone()` |
| **Fetch many** | `cur.fetchmany(n)` | `await cur.fetchmany(n)` |
| **Fetch all** | `cur.fetchall()` | `await cur.fetchall()` |
| **Iterate** | `for row in cur:` | `async for row in cur:` |
| **Commit** | `conn.commit()` | `await conn.commit()` |
| **Rollback** | `conn.rollback()` | `await conn.rollback()` |
| **Close** | `conn.close()` | `await conn.close()` |

---

## Quick Examples

### Simple Query

```python
import asyncio
from snowflake.connector.aio import connect

async def query_data():
    async with connect(user='...', password='...', account='...') as conn:
        async with conn.cursor() as cur:
            await cur.execute("SELECT * FROM table WHERE id < %s", (100,))
            async for row in cur:
                print(row)

asyncio.run(query_data())
```

### Concurrent Queries

```python
import asyncio
from snowflake.connector.aio import connect

async def fetch(conn, sql):
    cur = conn.cursor()
    await cur.execute(sql)
    return await cur.fetchall()

async def main():
    async with connect(user='...', password='...', account='...') as conn:
        results = await asyncio.gather(
            fetch(conn, "SELECT * FROM table1"),
            fetch(conn, "SELECT * FROM table2"),
        )
        table1_rows, table2_rows = results

asyncio.run(main())
```

### Dictionary Cursor

```python
from snowflake.connector.aio import connect, DictCursor

async def main():
    async with connect(user='...', password='...', account='...') as conn:
        async with conn.cursor(DictCursor) as cur:
            await cur.execute("SELECT id, name FROM users")
            async for row in cur:
                print(row["ID"], row["NAME"])

asyncio.run(main())
```



---

## Common Pitfalls

### Using `SnowflakeConnection` without calling `connect()`

```python
# WRONG — connection not established yet
conn = SnowflakeConnection(user='...', password='...', account='...')
cur = conn.cursor()
await cur.execute("SELECT 1")  # Will fail!

# CORRECT — call connect() first
conn = SnowflakeConnection(user='...', password='...', account='...')
await conn.connect()
cur = conn.cursor()
await cur.execute("SELECT 1")
```

### Running queries sequentially instead of concurrently

```python
# SLOW — sequential awaits
async with connect(...) as conn:
    cur1 = conn.cursor()
    await cur1.execute("SELECT * FROM table1")
    results1 = await cur1.fetchall()

    cur2 = conn.cursor()
    await cur2.execute("SELECT * FROM table2")
    results2 = await cur2.fetchall()

# FAST — concurrent with asyncio.gather
async def fetch(conn, table):
    cur = conn.cursor()
    await cur.execute(f"SELECT * FROM {table}")
    return await cur.fetchall()

async with connect(...) as conn:
    results1, results2 = await asyncio.gather(
        fetch(conn, 'table1'),
        fetch(conn, 'table2'),
    )
```
