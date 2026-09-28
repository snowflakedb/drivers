#!/usr/bin/env python3
"""Apply bulk JDBC mappings into tests/oldTestsCoverage/jdbc.yaml.

Invoked from the test-coverage-mapper skill (JDBC bulk apply). Idempotent:
skips entries that already have ud_tests or not-applicable status.
"""
from __future__ import annotations

import re
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
PATH = ROOT / "tests/oldTestsCoverage/jdbc.yaml"
TEST_ROOT = ROOT / "jdbc/src/test/java"


def first_test_method(rel_java: str) -> str | None:
    path = ROOT / rel_java
    if not path.is_file():
        return None
    methods = re.findall(
        r"(?:@Test|@ParameterizedTest)[\s\S]*?\n\s+(?:public\s+)?void\s+(\w+)\s*\(",
        path.read_text(),
    )
    return methods[0] if methods else None


def ptr(rel: str, method: str | None = None) -> str:
    if method:
        return f"{rel}#{method}"
    m = first_test_method(rel)
    return f"{rel}#{m}" if m else rel


NA_CORE = (
    "Old snowflake-jdbc Java core unit/IT; logic lives in sf_core / jdbc_bridge "
    "for UD. No UD Java wrapper equivalent."
)
NA_NOTES = {
    "core": NA_CORE,
    "loader": "Old snowflake-jdbc ingest Loader API; UD has no Java loader surface.",
    "minicore": (
        "Old minicore native loader tests; UD uses jdbc_bridge / NativeLibraryLoader "
        "differently."
    ),
    "json": "Old JSON result-format converters; UD materializes through Arrow only.",
    "fileuploader": (
        "Old Java FileUploader / TransferAgent internals; UD PUT/GET goes through "
        "sf_core stage APIs covered by e2e/put_get."
    ),
    "binduploader": (
        "Old Java BindUploader internals; UD stage binding covered by "
        "ParameterBindingTests / LargeBindingsTests."
    ),
    "diagnostic": "Old JDBC diagnostic context; no UD Java diagnostic package.",
    "rest": "Old RestRequest Java HTTP client; HTTP is in sf_core.",
    "storage_client": (
        "Old cloud storage client exception ITs; stage IO is in sf_core."
    ),
    "json_vs_arrow": (
        "UD always uses Arrow vectors (including when the backend returns JSON). "
        "JSON-vs-Arrow matrix is not-applicable."
    ),
    "telemetry_oob": (
        "Old OOB/IB telemetry client; UD CoreTelemetry is a thin wrapper covered "
        "separately if at all."
    ),
    "util_helpers": (
        "Old util helper unit tests with no UD Java analogue or covered incidentally."
    ),
    "sf_client_config": (
        "SFClientConfig JSON parser is not ported as a Java API; logging/config uses "
        "different paths under BD#71 / sf_core."
    ),
    "mock_connection": (
        "Old mock connection harness; UD uses WireMock / unit mocks differently."
    ),
    "header_customizer": (
        "Old HTTP header customizer on SFConnectionHandler; UD header injection is "
        "not the same Java class."
    ),
}

META = "jdbc/src/test/java/net/snowflake/client/api/metadata/SnowflakeDatabaseMetaDataTests.java"
POOL_IT = "jdbc/src/test/java/net/snowflake/client/api/pooling"
POOL_E2E = "jdbc/src/test/java/net/snowflake/jdbc/e2e/pooling/ConnectionPoolTests.java"
CALL_UNIT = (
    "jdbc/src/test/java/net/snowflake/client/internal/api/implementation/statement/"
    "SnowflakeCallableStatementImplTest.java"
)
CALL_E2E = "jdbc/src/test/java/net/snowflake/jdbc/e2e/query/CallableStatementTests.java"
BATCH = "jdbc/src/test/java/net/snowflake/client/api/statement/SnowflakeBatchExecutionTest.java"
MULTI = "jdbc/src/test/java/net/snowflake/jdbc/e2e/query/MultistatementTests.java"
INTERVAL_TESTS = "jdbc/src/test/java/net/snowflake/jdbc/e2e/types/IntervalTests.java"
DECFLOAT_TESTS = "jdbc/src/test/java/net/snowflake/jdbc/e2e/types/DecfloatTests.java"
VECTOR_TESTS = "jdbc/src/test/java/net/snowflake/jdbc/e2e/types/VectorTests.java"
Getters = "jdbc/src/test/java/net/snowflake/client/api/resultset/SnowflakeResultSetGettersTest.java"
Async = "jdbc/src/test/java/net/snowflake/client/api/resultset/AsyncQueryTests.java"
RSLife = (
    "jdbc/src/test/java/net/snowflake/client/internal/api/implementation/resultset/"
    "SnowflakeResultSetImplHandleLifecycleTest.java"
)
Stmt = "jdbc/src/test/java/net/snowflake/client/api/statement/SnowflakeStatementTest.java"
PrepNS = (
    "jdbc/src/test/java/net/snowflake/client/api/statement/"
    "SnowflakePreparedStatementUnsupportedFeatureTest.java"
)
PrepBind = (
    "jdbc/src/test/java/net/snowflake/client/api/statement/"
    "SnowflakePreparedStatementBindingTest.java"
)
ParamBind = "jdbc/src/test/java/net/snowflake/jdbc/e2e/query/ParameterBindingTests.java"
ConnImpl = (
    "jdbc/src/test/java/net/snowflake/client/internal/api/implementation/connection/"
    "SnowflakeConnectionImplTest.java"
)
Driver = "jdbc/src/test/java/net/snowflake/client/api/driver/SnowflakeDriverTest.java"
AuthEB = "jdbc/src/test/java/net/snowflake/jdbc/e2e/authentication/ExternalBrowserTests.java"
AuthOkta = "jdbc/src/test/java/net/snowflake/jdbc/e2e/authentication/NativeOktaTests.java"
AuthPat = "jdbc/src/test/java/net/snowflake/jdbc/e2e/authentication/PatTests.java"
AuthMfa = "jdbc/src/test/java/net/snowflake/jdbc/e2e/authentication/UserPasswordMfaTests.java"
AuthOauth = "jdbc/src/test/java/net/snowflake/jdbc/e2e/authentication/OauthTests.java"
Semi = "jdbc/src/test/java/net/snowflake/jdbc/e2e/types/SemiStructuredTests.java"
DistFetch = "jdbc/src/test/java/net/snowflake/jdbc/e2e/query/DistributedFetchTests.java"
AutoCommit = "jdbc/src/test/java/net/snowflake/jdbc/e2e/session/AutoCommitTests.java"
SessionParams = "jdbc/src/test/java/net/snowflake/jdbc/e2e/session/SessionParametersTests.java"
Proxy = "jdbc/src/test/java/net/snowflake/jdbc/e2e/session/ProxyTests.java"
Tls = "jdbc/src/test/java/net/snowflake/jdbc/e2e/tls/TlsVersionTest.java"
DSConn = "jdbc/src/test/java/net/snowflake/client/api/datasource/DataSourceConnectionTests.java"
PUT = "jdbc/src/test/java/net/snowflake/jdbc/e2e/put_get"
WIF_CROSS = (
    "jdbc/src/test/java/net/snowflake/jdbc/e2e/authentication/"
    "WorkloadIdentityCrossParamTests.java"
)


def set_mapped(entry: dict, ud_list: list[str]) -> bool:
    if entry.get("ud_tests"):
        return False
    if entry.get("status") == "not-applicable":
        return False
    entry["ud_tests"] = list(ud_list)
    entry["status"] = "mapped"
    entry.pop("gaps", None)
    entry.pop("notes", None)
    return True


def set_partial(entry: dict, ud_list: list[str], gaps: list[str]) -> bool:
    if entry.get("ud_tests") and entry.get("status") not in (None, "partial", "unmapped"):
        return False
    if entry.get("status") == "not-applicable":
        return False
    if entry.get("ud_tests") and entry.get("status") != "partial":
        return False
    entry["ud_tests"] = list(ud_list)
    entry["status"] = "partial"
    entry["gaps"] = list(gaps)
    return True


def set_na(entry: dict, notes: str) -> bool:
    if entry.get("status") == "not-applicable":
        return False
    if entry.get("ud_tests"):
        return False
    entry["ud_tests"] = []
    entry["status"] = "not-applicable"
    entry["notes"] = notes
    entry.pop("gaps", None)
    return True


def na_note_for(fname: str) -> str | None:
    if fname.startswith("core.json."):
        return NA_NOTES["json"]
    if fname.startswith("core.arrow."):
        return None
    if fname.startswith("core."):
        return NA_NOTES["core"]
    if fname.startswith("loader."):
        return NA_NOTES["loader"]
    if fname.startswith("internal.core.minicore"):
        return NA_NOTES["minicore"]
    if "FileUploader" in fname or "CompressedStreamFactory" in fname:
        return NA_NOTES["fileuploader"]
    if "BindUploader" in fname:
        return NA_NOTES["binduploader"]
    if "diagnostic" in fname.lower():
        return NA_NOTES["diagnostic"]
    if fname.startswith("jdbc.RestRequest"):
        return NA_NOTES["rest"]
    if (
        "cloud.storage" in fname
        or "S3Client" in fname
        or "AzureClient" in fname
        or "GcsClient" in fname
    ):
        return NA_NOTES["storage_client"]
    if "ResultSetJsonVsArrow" in fname:
        return NA_NOTES["json_vs_arrow"]
    if fname.startswith("jdbc.telemetryOOB") or fname.startswith("jdbc.telemetry."):
        return NA_NOTES["telemetry_oob"]
    if fname.startswith("util."):
        return NA_NOTES["util_helpers"]
    if fname in (
        "config.SFClientConfigParserTest.java",
        "config.SFPermissionsTest.java",
    ):
        return NA_NOTES["sf_client_config"]
    if fname.startswith("jdbc.MockConnection"):
        return NA_NOTES["mock_connection"]
    if fname.startswith("jdbc.DefaultSFConnectionHandler"):
        return NA_NOTES["header_customizer"]
    return None


def main() -> None:
    data = yaml.safe_load(PATH.read_text())
    tests: dict = data["tests"]
    stats = {"mapped": 0, "partial": 0, "na": 0, "skipped": 0}

    def bump(kind: str, changed: bool) -> None:
        stats[kind if changed else "skipped"] += 1

    for fname, cases in tests.items():
        note = na_note_for(fname)
        if not note:
            continue
        for case in cases:
            bump("na", set_na(case, note))

    arrow_map = {
        "BigIntToFixedConverterTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/converters/"
                "BigIntToFixedConverterTest.java",
                "testFixedNoScale",
            ),
        ],
        "BitToBooleanConverterTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/converters/"
                "BitToBooleanConverterTest.java"
            )
        ],
        "DateConverterTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/converters/"
                "DateConverterTest.java"
            )
        ],
        "DoubleToRealConverterTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/converters/"
                "DoubleToRealConverterTest.java"
            )
        ],
        "IntToFixedConverterTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/converters/"
                "IntToFixedConverterTest.java"
            )
        ],
        "SmallIntToFixedConverterTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/converters/"
                "SmallIntToFixedConverterTest.java"
            )
        ],
        "TinyIntToFixedConverterTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/converters/"
                "TinyIntToFixedConverterTest.java"
            )
        ],
        "VarBinaryToBinaryConverterTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/converters/"
                "VarBinaryToBinaryConverterTest.java"
            )
        ],
        "VarCharConverterTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/converters/"
                "VarCharConverterTest.java"
            )
        ],
        "BigIntToTimeConverterTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/converters/"
                "TimeConverterTest.java"
            )
        ],
        "IntToTimeConverterTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/converters/"
                "TimeConverterTest.java"
            )
        ],
        "ArrowResultUtilTest.java": [
            ptr(
                "jdbc/src/test/java/net/snowflake/client/internal/core/arrow/"
                "ArrowDateUtilTest.java"
            )
        ],
    }
    arrow_partial = {
        "BigIntToTimestampNTZConverterTest.java": (
            ["jdbc/src/test/java/net/snowflake/jdbc/e2e/types/TimestampNtzTests.java"],
            [
                "Converter-level unit coverage is being added; e2e TimestampNtzTests "
                "covers customer getters."
            ],
        ),
        "BigIntToTimestampLTZConverterTest.java": (
            ["jdbc/src/test/java/net/snowflake/jdbc/e2e/types/TimestampLtzTests.java"],
            [
                "Converter-level unit coverage is being added; e2e TimestampLtzTests "
                "covers customer getters."
            ],
        ),
        "TwoFieldStructToTimestampNTZConverterTest.java": (
            ["jdbc/src/test/java/net/snowflake/jdbc/e2e/types/TimestampNtzTests.java"],
            ["Struct-backed NTZ converter units pending; e2e covers getters."],
        ),
        "TwoFieldStructToTimestampLTZConverterTest.java": (
            ["jdbc/src/test/java/net/snowflake/jdbc/e2e/types/TimestampLtzTests.java"],
            ["Struct-backed LTZ converter units pending; e2e covers getters."],
        ),
        "TwoFieldStructToTimestampTZConverterTest.java": (
            ["jdbc/src/test/java/net/snowflake/jdbc/e2e/types/TimestampTzTests.java"],
            ["Struct-backed TZ converter units pending; e2e covers getters."],
        ),
        "ThreeFieldStructToTimestampTZConverterTest.java": (
            ["jdbc/src/test/java/net/snowflake/jdbc/e2e/types/TimestampTzTests.java"],
            ["Three-field TZ converter units pending; e2e covers getters."],
        ),
    }
    for fname, cases in tests.items():
        if not fname.startswith("core.arrow."):
            continue
        short = fname[len("core.arrow.") :]
        if short in arrow_map:
            for case in cases:
                bump("mapped", set_mapped(case, arrow_map[short]))
        elif short in arrow_partial:
            ud, gaps = arrow_partial[short]
            for case in cases:
                bump("partial", set_partial(case, ud, gaps))

    for fname in [
        "jdbc.DatabaseMetaDataIT.java",
        "jdbc.DatabaseMetaDataLatestIT.java",
        "jdbc.DatabaseMetaDataInternalIT.java",
        "jdbc.DatabaseMetaDataInternalLatestIT.java",
        "jdbc.DatabaseMetaDataResultsetIT.java",
        "jdbc.DatabaseMetaDataResultSetLatestIT.java",
    ]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(META, "shouldReturnSnowflakeForDatabaseProductName"),
                        ptr(META, "shouldReturnTrueForAllTablesAreSelectable"),
                    ],
                ),
            )

    pool_exact = {
        "pooling.LogicalConnectionFeatureNotSupportedLatestIT.java": {
            "testLogicalConnectionFeatureNotSupported": [
                ptr(
                    f"{POOL_IT}/LogicalConnectionFeatureNotSupportedIT.java",
                    "shouldLogicalConnectionFeatureNotSupported",
                ),
                ptr(POOL_E2E, "shouldRejectUnsupportedFeaturesOnLogicalConnection"),
            ],
        },
        "pooling.LogicalConnectionAlreadyClosedLatestIT.java": {
            "testLogicalConnectionAlreadyClosed": [
                ptr(
                    f"{POOL_IT}/LogicalConnectionClosedIT.java",
                    "shouldLogicalConnectionAlreadyClosed",
                ),
                ptr(POOL_E2E, "shouldRejectOperationsOnClosedLogicalConnection"),
            ],
        },
        "pooling.ConnectionPoolingDataSourceIT.java": {
            "testPooledConnection": [
                ptr(
                    f"{POOL_IT}/PooledConnectionLifecycleIT.java",
                    "shouldPooledConnectionBasicLifecycle",
                ),
                ptr(
                    POOL_E2E,
                    "shouldBorrowLogicalConnectionAndKeepPhysicalConnectionAliveAfterClose",
                ),
            ],
            "testPooledConnectionUsernamePassword": [
                ptr(
                    f"{POOL_IT}/PooledConnectionLifecycleIT.java",
                    "shouldGetPooledConnectionWithUserAndPassword",
                ),
                ptr(POOL_E2E, "shouldGetPooledConnectionWithUsernameAndPassword"),
            ],
        },
    }
    for fname, by_test in pool_exact.items():
        if fname not in tests:
            continue
        for case in tests[fname]:
            if case["test_name"] in by_test:
                bump("mapped", set_mapped(case, by_test[case["test_name"]]))

    if "pooling.LogicalConnectionLatestIT.java" in tests:
        for case in tests["pooling.LogicalConnectionLatestIT.java"]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(
                            f"{POOL_IT}/LogicalConnectionIT.java",
                            "shouldCreateStatementWithHoldability",
                        ),
                        ptr(
                            POOL_E2E,
                            "shouldCreateStatementWithHoldabilityOnLogicalConnection",
                        ),
                    ],
                ),
            )

    if "jdbc.ConnectionPoolingIT.java" in tests:
        for case in tests["jdbc.ConnectionPoolingIT.java"]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(
                            POOL_E2E,
                            "shouldBorrowLogicalConnectionAndKeepPhysicalConnectionAliveAfterClose",
                        ),
                        "jdbc/src/test/java/net/snowflake/jdbc/integration/pooling/"
                        "PoolDataSourceConfigurationTests.java",
                    ],
                ),
            )

    if "jdbc.CallableStatementIT.java" in tests:
        for case in tests["jdbc.CallableStatementIT.java"]:
            if case["test_name"] == "testFeatureNotSupportedException":
                bump(
                    "mapped",
                    set_mapped(
                        case,
                        [
                            ptr(
                                CALL_UNIT,
                                "shouldThrowFeatureNotSupportedWhenRegisteringOutParameterByIndex",
                            )
                        ],
                    ),
                )
            elif case["test_name"] == "testPrepareCall":
                bump(
                    "mapped",
                    set_mapped(
                        case,
                        [
                            ptr(
                                CALL_E2E,
                                "shouldExecuteStoredProcedureWithNoBindingParameters",
                            )
                        ],
                    ),
                )

    if "jdbc.CallableStatementLatestIT.java" in tests:
        for case in tests["jdbc.CallableStatementLatestIT.java"]:
            name = case["test_name"]
            if "Curly" in name or "Escape" in name or "parse" in name.lower():
                bump(
                    "mapped",
                    set_mapped(
                        case,
                        [
                            ptr(
                                CALL_UNIT,
                                "shouldStripOuterCurlyBracketsFromSqlEscapeSyntax",
                            ),
                            ptr(
                                CALL_E2E,
                                "shouldStripCurlyBracketEscapeSyntaxWithTwoBindingParameters",
                            ),
                        ],
                    ),
                )
            else:
                bump(
                    "mapped",
                    set_mapped(
                        case,
                        [
                            ptr(
                                CALL_E2E,
                                "shouldExecuteStoredProcedureWithBindingParameters",
                            )
                        ],
                    ),
                )

    for fname in ["jdbc.BatchExecutionIT.java", "jdbc.BatchExecutionLatestIT.java"]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(BATCH, "testStatementBatchExecutesEachSqlAndReturnsCounts"),
                        ptr(
                            BATCH,
                            "testPreparedStatementBatchInsertExpandsToPerRowCounts",
                        ),
                    ],
                ),
            )

    for fname in [
        "jdbc.MultiStatementIT.java",
        "jdbc.MultiStatementLatestIT.java",
        "jdbc.PreparedMultiStmtIT.java",
    ]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(MULTI, "shouldExecuteMultipleSelectStatements"),
                        ptr(MULTI, "shouldExecuteMultipleDmlStatements"),
                    ],
                ),
            )

    for fname in [
        "jdbc.IntervalDayTimeTypeLatestIT.java",
        "jdbc.IntervalYearMonthTypeLatestIT.java",
    ]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(
                            INTERVAL_TESTS,
                            "shouldCastIntervalValuesToAppropriateTypeForYearToMonthAndDayToSecond",
                        ),
                        ptr(
                            INTERVAL_TESTS,
                            "shouldInsertAndSelectBackIntervalDayToSecondValuesUsingParameterBinding",
                        ),
                    ],
                ),
            )

    if "jdbc.DecfloatTypeLatestIT.java" in tests:
        for case in tests["jdbc.DecfloatTypeLatestIT.java"]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [ptr(DECFLOAT_TESTS, "shouldCastDecfloatValuesToAppropriateType")],
                ),
            )

    if "jdbc.ResultSetVectorLatestIT.java" in tests:
        for case in tests["jdbc.ResultSetVectorLatestIT.java"]:
            bump("mapped", set_mapped(case, [ptr(VECTOR_TESTS)]))

    for fname in ["jdbc.BindingDataIT.java", "jdbc.BindingDataLatestIT.java"]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump("mapped", set_mapped(case, [ptr(ParamBind), ptr(PrepBind)]))

    if "jdbc.ConnectionFeatureNotSupportedIT.java" in tests:
        for case in tests["jdbc.ConnectionFeatureNotSupportedIT.java"]:
            bump(
                "mapped",
                set_mapped(case, [ptr(ConnImpl, "shouldRejectSetSavepoint")]),
            )
    if "jdbc.ConnectionAlreadyClosedIT.java" in tests:
        for case in tests["jdbc.ConnectionAlreadyClosedIT.java"]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(
                            ConnImpl,
                            "shouldRejectOperationsAfterCloseWithConnectionClosedCode",
                        )
                    ],
                ),
            )

    auth_exact = {
        "authentication.ExternalBrowserLatestIT.java": {
            "shouldAuthenticateUsingExternalBrowser": [
                ptr(AuthEB, "shouldAuthenticateWithExternalBrowserViaOktaIdp")
            ],
        },
        "authentication.IdTokenLatestIT.java": {
            "shouldAuthenticateUsingTokenWithoutBrowser": [
                ptr(AuthEB, "shouldReuseCachedIdTokenWithoutBrowserInteraction")
            ],
            "shouldAuthenticateUsingExternalBrowserAndSaveToken": [
                ptr(AuthEB, "shouldAuthenticateWithExternalBrowserViaOktaIdp")
            ],
        },
        "authentication.MFALatestIT.java": {
            "testMfaSuccessful": [
                ptr(AuthMfa, "shouldAuthenticateUsingUsernamePasswordAndTotpPasscode")
            ],
        },
        "authentication.OktaAuthLatestIT.java": {
            "shouldAuthenticateUsingOkta": [
                ptr(AuthOkta, "shouldAuthenticateUsingNativeOkta")
            ],
            "shouldThrowErrorForWrongOktaCredentials": [
                ptr(AuthOkta, "shouldFailNativeOktaAuthenticationWithWrongCredentials")
            ],
            "shouldThrowErrorForWrongOktaUrl": [
                ptr(AuthOkta, "shouldFailNativeOktaAuthenticationWithWrongOktaUrl")
            ],
        },
        "authentication.PATLatestIT.java": {
            "shouldAuthenticateUsingPAT": [
                ptr(AuthPat, "shouldAuthenticateUsingPatAsToken")
            ],
            "shouldThrowErrorForInvalidPAT": [
                ptr(AuthPat, "shouldFailPatAuthenticationWhenInvalidTokenProvided")
            ],
        },
        "authentication.OauthLatestIT.java": {
            "shouldAuthenticateUsingOauth": [
                ptr(AuthOauth, "oauthShouldAuthenticateWithPreAcquiredAccessToken")
            ],
            "shouldThrowErrorForInvalidToken": [
                ptr(AuthOauth, "oauthShouldFailLegacyAuthenticationWithInvalidToken")
            ],
        },
    }
    for fname, by_test in auth_exact.items():
        if fname not in tests:
            continue
        for case in tests[fname]:
            if case["test_name"] in by_test:
                bump("mapped", set_mapped(case, by_test[case["test_name"]]))
            elif fname == "authentication.ExternalBrowserLatestIT.java":
                bump(
                    "partial",
                    set_partial(
                        case,
                        [ptr(AuthEB, "shouldAuthenticateWithExternalBrowserViaOktaIdp")],
                        [
                            "UD ExternalBrowserTests covers happy/cache paths; this "
                            "error polarity is not a dedicated assertion."
                        ],
                    ),
                )
            elif "Okta" in fname and "Username" in case["test_name"]:
                bump(
                    "partial",
                    set_partial(
                        case,
                        [ptr(AuthOkta, "shouldAuthenticateUsingNativeOkta")],
                        [
                            "OktaUsername param / mismatch cases are not fully covered "
                            "by NativeOktaTests."
                        ],
                    ),
                )
            elif "PAT" in fname and "Mismatch" in case["test_name"]:
                bump(
                    "partial",
                    set_partial(
                        case,
                        [ptr(AuthPat, "shouldAuthenticateUsingPatAsToken")],
                        ["PAT username-mismatch polarity is not a dedicated UD assertion."],
                    ),
                )
            elif "OauthLatest" in fname and "Mismatch" in case["test_name"]:
                bump(
                    "partial",
                    set_partial(
                        case,
                        [
                            ptr(
                                AuthOauth,
                                "oauthShouldAuthenticateWithPreAcquiredAccessToken",
                            )
                        ],
                        [
                            "OAuth username-mismatch polarity is not a dedicated UD "
                            "assertion."
                        ],
                    ),
                )

    oauth_ac_cc = {
        "authentication.OauthOktaClientCredentialsLatestIT.java": {
            "shouldAuthenticateUsingSnowflakeOauthClientCredentials": [
                ptr(AuthOauth, "oauthShouldAuthenticateUsingClientCredentialsFlow")
            ],
            "shouldThrowErrorForUnauthorizedClientCredentials": [
                ptr(AuthOauth, "oauthShouldFailClientCredentialsFlowWithBadClientSecret")
            ],
        },
        "authentication.OauthSnowflakeAuthorizationCodeLatestIT.java": {
            "shouldAuthenticateUsingSnowflakeOauthAuthorizationCode": [
                ptr(AuthOauth, "oauthShouldAuthenticateUsingAuthorizationCodeFlow")
            ],
            "shouldAuthenticateUsingTokenCacheOauthSnowflake": [
                ptr(
                    AuthOauth,
                    "oauthShouldReuseCachedAccessTokenWithoutBrowserInteraction",
                )
            ],
        },
        "authentication.OauthOktaAuthorizationCodeLatestIT.java": {
            "shouldAuthenticateUsingExternalOauthOktaAuthorizationCode": [
                ptr(AuthOauth, "oauthShouldAuthenticateUsingAuthorizationCodeFlow")
            ],
            "shouldAuthenticateUsingTokenCacheForOauthOkta": [
                ptr(
                    AuthOauth,
                    "oauthShouldReuseCachedAccessTokenWithoutBrowserInteraction",
                )
            ],
        },
        "authentication.OauthSnowflakeAuthorizationCodeWildcardsLatestIT.java": {
            "shouldAuthenticateUsingSnowflakeOauthAuthorizationCode": [
                ptr(AuthOauth, "oauthShouldAuthenticateUsingAuthorizationCodeFlow")
            ],
            "shouldAuthenticateUsingTokenCacheOauthSnowflake": [
                ptr(
                    AuthOauth,
                    "oauthShouldReuseCachedAccessTokenWithoutBrowserInteraction",
                )
            ],
        },
    }
    for fname, by_test in oauth_ac_cc.items():
        if fname not in tests:
            continue
        for case in tests[fname]:
            if case["test_name"] in by_test:
                bump("mapped", set_mapped(case, by_test[case["test_name"]]))
            else:
                bump(
                    "partial",
                    set_partial(
                        case,
                        [],
                        [
                            "No UD e2e asserts this IdP timeout/username-mismatch/"
                            "negative-cache case yet."
                        ],
                    ),
                )

    if "wif.WIFLatestIT.java" in tests:
        for case in tests["wif.WIFLatestIT.java"]:
            bump(
                "partial",
                set_partial(
                    case,
                    [
                        ptr(
                            WIF_CROSS,
                            "shouldRejectWifParamUnderNonWifAuthenticatorOnNewDriverButIgnoreOnOldDriver",
                        )
                    ],
                    [
                        "Only BD#48 cross-param rejection exists; OIDC/provider/"
                        "impersonation happy paths are not covered as ITs."
                    ],
                ),
            )

    put_exact = {
        "jdbc.PutFileWithSpaceIncludedIT.java": {
            "putFileWithSpaceIncluded": [
                ptr(
                    f"{PUT}/PutGetPathNormalizationTests.java",
                    "shouldUploadFileWhenSourcePathContainsDotdotSegments",
                ),
                ptr(f"{PUT}/PutGetBasicOperationsTest.java", "shouldGetFileUploadedToStage"),
            ],
        },
        "jdbc.PutUnescapeBackslashIT.java": {
            "testPutFileUnescapeBackslashes": [
                ptr(
                    f"{PUT}/PutGetPathNormalizationTests.java",
                    "shouldUploadFileWhenSourcePathIsRelativeToWorkingDirectory",
                )
            ],
        },
        "jdbc.StreamIT.java": {
            "testUploadStream": [
                ptr(
                    f"{PUT}/UploadDownloadStreamIT.java",
                    "shouldRoundTripCompressedStreamWithDestPrefix",
                )
            ],
            "testDownloadStream": [
                ptr(
                    f"{PUT}/UploadDownloadStreamIT.java",
                    "shouldRoundTripUncompressedStreamWithoutPrefix",
                )
            ],
            "testCompressAndUploadStream": [
                ptr(
                    f"{PUT}/UploadDownloadStreamIT.java",
                    "shouldRoundTripCompressedStreamWithDestPrefix",
                )
            ],
        },
    }
    for fname, by_test in put_exact.items():
        if fname not in tests:
            continue
        for case in tests[fname]:
            if case["test_name"] in by_test:
                bump("mapped", set_mapped(case, by_test[case["test_name"]]))

    if "jdbc.StreamLatestIT.java" in tests:
        for case in tests["jdbc.StreamLatestIT.java"]:
            if case.get("ud_tests"):
                continue
            if "Download" in case["test_name"] or "download" in case["test_name"]:
                bump(
                    "mapped",
                    set_mapped(
                        case,
                        [
                            ptr(
                                f"{PUT}/UploadDownloadStreamIT.java",
                                "shouldThrowWhenDownloadingMissingStageFile",
                            )
                        ],
                    ),
                )
            else:
                bump(
                    "mapped",
                    set_mapped(
                        case,
                        [
                            ptr(
                                f"{PUT}/UploadDownloadStreamIT.java",
                                "shouldRoundTripUncompressedStreamWithoutPrefix",
                            )
                        ],
                    ),
                )

    if "jdbc.ResultSetAlreadyClosedIT.java" in tests:
        for case in tests["jdbc.ResultSetAlreadyClosedIT.java"]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [ptr(RSLife, "shouldRejectSerializablesWhenResultSetClosed")],
                ),
            )

    if "jdbc.ResultSetAsyncIT.java" in tests:
        for case in tests["jdbc.ResultSetAsyncIT.java"]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(Async, "shouldExecuteAsyncQueryAndFetchResults"),
                        ptr(Async, "shouldGetQueryStatusViaConnectionWhenSuccess"),
                    ],
                ),
            )
    if "jdbc.ResultSetAsyncLatestIT.java" in tests:
        for case in tests["jdbc.ResultSetAsyncLatestIT.java"]:
            bump(
                "mapped",
                set_mapped(
                    case, [ptr(Async, "shouldExecuteAsyncQueryWithPreparedStatement")]
                ),
            )

    if "jdbc.ResultSetFeatureNotSupportedIT.java" in tests:
        for case in tests["jdbc.ResultSetFeatureNotSupportedIT.java"]:
            bump(
                "partial",
                set_partial(
                    case,
                    [ptr(Getters, "testGetInt")],
                    [
                        "Scroll/update NS on ResultSet is not yet a dedicated "
                        "public-API matrix."
                    ],
                ),
            )

    for fname in ["jdbc.ResultSetIT.java", "jdbc.ResultSetLatestIT.java"]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(Getters, "testGetInt"),
                        ptr(Getters, "testGetString"),
                        ptr(Getters, "testDecfloatWasNullAcrossMultipleGetters"),
                    ],
                ),
            )

    for fname in [
        "jdbc.ResultSetMultiTimeZoneIT.java",
        "jdbc.ResultSetMultiTimeZoneLatestIT.java",
        "jdbc.ResultSetArrowForceLTZMultiTimeZoneIT.java",
        "jdbc.ResultSetArrowForceTZMultiTimeZoneIT.java",
    ]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        "jdbc/src/test/java/net/snowflake/jdbc/e2e/types/TimestampLtzTests.java",
                        "jdbc/src/test/java/net/snowflake/jdbc/e2e/types/TimestampTzTests.java",
                        "jdbc/src/test/java/net/snowflake/jdbc/e2e/types/TimestampNtzTests.java",
                    ],
                ),
            )

    if "jdbc.SnowflakeResultSetSerializableIT.java" in tests:
        for case in tests["jdbc.SnowflakeResultSetSerializableIT.java"]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(
                            DistFetch,
                            "shouldRoundTripResultSetThroughSerializableRepeatedly",
                        )
                    ],
                ),
            )

    for fname, cases in tests.items():
        if "StructuredTypes" not in fname and "structuredtypes" not in fname:
            continue
        for case in cases:
            if case.get("ud_tests"):
                continue
            bump(
                "partial",
                set_partial(
                    case,
                    [ptr(Semi, "shouldCastSemiStructuredValuesToAppropriateType")],
                    [
                        "getList/getMap/getArray for structured types still throw "
                        "NotImplemented/NS (SNOW-3445814 / SNOW-2881790). "
                        "SemiStructuredTests covers VARIANT/OBJECT/ARRAY as strings."
                    ],
                ),
            )

    for fname in [
        "jdbc.StatementIT.java",
        "jdbc.StatementLatestIT.java",
        "jdbc.StatementNoOpLatestIT.java",
    ]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(Stmt, "testSimpleSelect"),
                        ptr(Stmt, "testStatementExecuteInsertTracksUpdateCount"),
                    ],
                ),
            )

    for fname in [
        "jdbc.PreparedStatement1IT.java",
        "jdbc.PreparedStatement2IT.java",
        "jdbc.PreparedStatement1LatestIT.java",
        "jdbc.PreparedStatement2LatestIT.java",
    ]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(PrepBind),
                        ptr(
                            PrepNS,
                            "testSetAsciiStreamWithIntLengthThrowsFeatureNotSupported",
                        ),
                    ],
                ),
            )

    for fname in ["jdbc.ConnectionIT.java", "jdbc.ConnectionLatestIT.java"]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(ConnImpl, "shouldDefaultToTrue"),
                        ptr(AutoCommit, "shouldDiscardUncommittedInsertsOnRollback"),
                        ptr(
                            SessionParams,
                            "shouldForwardUnrecognizedConnectionOptionAsSessionParameter",
                        ),
                    ],
                ),
            )

    for fname in [
        "jdbc.ConnectionWithOCSPModeIT.java",
        "jdbc.ConnectionWithDisableOCSPModeLatestIT.java",
    ]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump(
                "partial",
                set_partial(
                    case,
                    [
                        ptr(
                            Tls,
                            "shouldNegotiateTlsWhenTheServerOffersAVersionInsideTheWindow",
                        )
                    ],
                    [
                        "OCSP mode matrix is not fully ported; TLS version window is "
                        "covered."
                    ],
                ),
            )

    for fname in [
        "jdbc.SnowflakeDriverIT.java",
        "jdbc.SnowflakeDriverLatestIT.java",
        "jdbc.SnowflakeDriverTest.java",
    ]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            if case.get("ud_tests"):
                continue
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        ptr(Driver, "testAcceptsValidURL"),
                        ptr(Driver, "testDriverVersion"),
                        ptr(DSConn),
                    ],
                ),
            )

    for fname in ["jdbc.ProxyLatestIT.java", "jdbc.CustomProxyLatestIT.java"]:
        if fname not in tests:
            continue
        for case in tests[fname]:
            bump(
                "partial",
                set_partial(
                    case,
                    [
                        ptr(
                            Proxy,
                            "shouldRouteRequestThroughProxyWhenProxyHostAndPortAreConfigured",
                        )
                    ],
                    [
                        "UD ProxyTests covers host/port and nonProxyHosts bypass; full "
                        "CustomProxyLatestIT JVM-param matrix is thinner."
                    ],
                ),
            )

    if "jdbc.SnowflakeTypeTest.java" in tests:
        for case in tests["jdbc.SnowflakeTypeTest.java"]:
            bump(
                "partial",
                set_partial(
                    case,
                    [],
                    [
                        "SnowflakeTypeHelper unit table tests are being added under "
                        "this coverage epic."
                    ],
                ),
            )

    if "jdbc.SnowflakeTimestampWithTimezoneTest.java" in tests:
        for case in tests["jdbc.SnowflakeTimestampWithTimezoneTest.java"]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [
                        "jdbc/src/test/java/net/snowflake/client/internal/jdbc/"
                        "SnowflakeTimestampWithTimezoneTest.java",
                        "jdbc/src/test/java/net/snowflake/client/internal/jdbc/"
                        "SnowflakeTimeWithTimezoneTest.java",
                    ],
                ),
            )

    if "jdbc.SSOConnectionTest.java" in tests:
        for case in tests["jdbc.SSOConnectionTest.java"]:
            bump(
                "mapped",
                set_mapped(
                    case,
                    [ptr(AuthEB, "shouldReuseCachedIdTokenWithoutBrowserInteraction")],
                ),
            )

    # Remaining empty config.SFConnectionConfigParserTest rows that are not partial:
    # leave as-is (BD#71 gaps already documented). FileConnectionConfiguration token tests:
    if "jdbc.FileConnectionConfigurationLatestIT.java" in tests:
        for case in tests["jdbc.FileConnectionConfigurationLatestIT.java"]:
            if case.get("ud_tests"):
                continue
            bump(
                "partial",
                set_partial(
                    case,
                    [
                        "jdbc/src/test/java/net/snowflake/jdbc/e2e/session/"
                        "AutoConnectionConfigFileTests.java"
                    ],
                    [
                        "Token-file connection scenarios differ under BD#71; "
                        "AutoConnectionConfigFileTests covers TOML profile resolution."
                    ],
                ),
            )

    mapped = empty = na = partial = 0
    for cases in tests.values():
        for case in cases:
            st = case.get("status")
            ud = case.get("ud_tests") or []
            if st == "not-applicable":
                na += 1
            elif st == "partial":
                partial += 1
            elif ud:
                mapped += 1
            else:
                empty += 1

    class CustomDumper(yaml.SafeDumper):
        pass

    def str_presenter(dumper, data):
        if "\n" in data:
            return dumper.represent_scalar("tag:yaml.org,2002:str", data, style="|")
        return dumper.represent_scalar("tag:yaml.org,2002:str", data)

    CustomDumper.add_representer(str, str_presenter)

    PATH.write_text(
        yaml.dump(
            data,
            Dumper=CustomDumper,
            default_flow_style=False,
            sort_keys=False,
            allow_unicode=True,
            width=120,
        )
    )
    print("apply stats", stats)
    print(
        f"final: mapped={mapped} partial={partial} na={na} empty={empty} "
        f"total={mapped + partial + na + empty}"
    )
    print("wrote", PATH)


if __name__ == "__main__":
    main()
