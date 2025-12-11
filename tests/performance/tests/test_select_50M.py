def test_select_string_50M_arrow(perf_test):
    perf_test(
        sql_command="SELECT L_COMMENT FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 50000000"
    )


def test_select_number_50M_arrow(perf_test):
    perf_test(
        sql_command="SELECT L_LINENUMBER::INT FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 50000000"
    )


def test_select_date_50M_arrow(perf_test):
    perf_test(
        sql_command="SELECT L_SHIPDATE FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 50000000"
    )


def test_select_float_50M_arrow(perf_test):
    perf_test(
        sql_command="SELECT L_EXTENDEDPRICE FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM LIMIT 50000000"
    )


def test_select_15columns_50M_arrow(perf_test):
    perf_test(
        sql_command="""
            SELECT 
                L_ORDERKEY,
                L_PARTKEY,
                L_SUPPKEY,
                L_LINENUMBER,
                L_QUANTITY,
                L_EXTENDEDPRICE,
                L_DISCOUNT,
                L_TAX,
                L_RETURNFLAG,
                L_LINESTATUS,
                L_SHIPDATE,
                L_COMMITDATE,
                L_RECEIPTDATE,
                L_SHIPINSTRUCT,
                L_COMMENT
            FROM SNOWFLAKE_SAMPLE_DATA.TPCH_SF100.LINEITEM 
            LIMIT 50000000
        """
    )

