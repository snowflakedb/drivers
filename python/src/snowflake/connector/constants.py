"""BACKWARD COMPATIBILITY MODULE ONLY"""

from collections import defaultdict

from .config_manager import CONFIG_FILE, CONNECTIONS_FILE  # noqa


# Maps Snowflake type IDs to type name strings.
# Values from snowflake-connector-python constants.py.
FIELD_ID_TO_NAME: defaultdict = defaultdict(
    str,
    {
        0: "FIXED",
        1: "REAL",
        2: "TEXT",
        3: "DATE",
        4: "TIMESTAMP",
        5: "VARIANT",
        6: "TIMESTAMP_LTZ",
        7: "TIMESTAMP_TZ",
        8: "TIMESTAMP_NTZ",
        9: "OBJECT",
        10: "ARRAY",
        11: "BINARY",
        12: "TIME",
        13: "BOOLEAN",
        14: "GEOGRAPHY",
        15: "GEOMETRY",
        16: "VECTOR",
        17: "MAP",
        18: "FILE",
        19: "INTERVAL_YEAR_MONTH",
        20: "INTERVAL_DAY_TIME",
    },
)

# Environment variable name for partner application identification.
ENV_VAR_PARTNER = "SF_PARTNER"

# UTF-8 encoding constant used by compat.py PKCS5_PAD.
UTF8 = "utf-8"
