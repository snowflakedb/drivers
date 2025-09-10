import sys
import os
sys.path.append("pep249_dbapi")
import pep249_dbapi

# Configure logging
import logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    handlers=[
        logging.StreamHandler()
    ]
)
logger = logging.getLogger(__name__)


password = os.getenv("PASSWORD")
if password is None:
    raise ValueError("PASSWORD environment variable is not set")

conn = pep249_dbapi.connect(
    database="testdb_universal",
    schema="public",
    user="test_universal",
    account="sfctest0",
    password=password,
    host="sfctest0.snowflakecomputing.com"
)

with conn:
    cursor = conn.cursor()
    cursor.execute("SELECT 1")
    result = cursor.fetchone()
    print(result)