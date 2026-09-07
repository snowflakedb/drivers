using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class BinaryTest : IClassFixture<ITFixture>
{
    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public BinaryTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should cast binary values to appropriate type
    [SnowflakeFact]
    public void ShouldCastBinaryValuesToAppropriateType()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY, TO_BINARY('V29ybGQ=', 'BASE64')::BINARY" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY, TO_BINARY('V29ybGQ=', 'BASE64')::BINARY";
        using var reader = cmd.ExecuteReader();

        // Then All values should be returned as appropriate binary type
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(typeof(byte[]), reader.GetFieldType(0));
        Assert.Equal(typeof(byte[]), reader.GetFieldType(1));

        // And the result should contain binary values:
        var col1 = (byte[])reader.GetValue(0);
        var col2 = (byte[])reader.GetValue(1);
        Assert.Equal("Hello"u8.ToArray(), col1);
        Assert.Equal("World"u8.ToArray(), col2);
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select binary literals
    [SnowflakeFact]
    public void ShouldSelectBinaryLiterals()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Queries selecting binary literals are executed:
        using var cmd1 = connection.CreateCommand();
        // SELECT X'48656C6C6F'::BINARY
        cmd1.CommandText = "SELECT X'48656C6C6F'::BINARY";
        using var reader1 = cmd1.ExecuteReader();
        Assert.True(reader1.Read(), "Expected one row");
        Assert.Equal("Hello"u8.ToArray(), (byte[])reader1.GetValue(0));

        // SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY
        using var cmd2 = connection.CreateCommand();
        cmd2.CommandText = "SELECT TO_BINARY('48656C6C6F', 'HEX')::BINARY";
        using var reader2 = cmd2.ExecuteReader();
        Assert.True(reader2.Read(), "Expected one row");
        Assert.Equal("Hello"u8.ToArray(), (byte[])reader2.GetValue(0));

        // SELECT TO_BINARY('ASNFZ4mrze8=', 'BASE64')::BINARY
        using var cmd3 = connection.CreateCommand();
        cmd3.CommandText = "SELECT TO_BINARY('ASNFZ4mrze8=', 'BASE64')::BINARY";
        using var reader3 = cmd3.ExecuteReader();

        // Then the results should contain expected binary values
        Assert.True(reader3.Read(), "Expected one row");
        Assert.Equal(new byte[] { 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF }, (byte[])reader3.GetValue(0));
    }

    // Scenario: should handle binary corner case values from literals
    [SnowflakeFact]
    public void ShouldHandleBinaryCornerCaseValuesFromLiterals()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query selecting corner case binary literals is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT X''::BINARY, X'00'::BINARY, X'FF'::BINARY, X'0000000000'::BINARY, X'FFFFFFFFFF'::BINARY, X'48006500'::BINARY";
        using var reader = cmd.ExecuteReader();

        // Then the result should contain expected corner case binary values
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(Array.Empty<byte>(), (byte[])reader.GetValue(0));
        Assert.Equal(new byte[] { 0x00 }, (byte[])reader.GetValue(1));
        Assert.Equal(new byte[] { 0xFF }, (byte[])reader.GetValue(2));
        Assert.Equal("\0\0\0\0\0"u8.ToArray(), (byte[])reader.GetValue(3));
        Assert.Equal(new byte[] { 0xFF, 0xFF, 0xFF, 0xFF, 0xFF }, (byte[])reader.GetValue(4));
        Assert.Equal("H\0e\0"u8.ToArray(), (byte[])reader.GetValue(5));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle NULL binary values from literals
    [SnowflakeFact]
    public void ShouldHandleNullBinaryValuesFromLiterals()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT NULL::{type}, X'ABCD', NULL::{type}" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT NULL::BINARY, X'ABCD', NULL::BINARY";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [NULL, 0xABCD, NULL]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for column 0");
        Assert.Equal(new byte[] { 0xAB, 0xCD }, (byte[])reader.GetValue(1));
        Assert.True(reader.IsDBNull(2), "Expected NULL for column 2");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select binary values from table
    [SnowflakeFact]
    public void ShouldSelectBinaryValuesFromTable()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And A temporary table with BINARY column is created
        var tableName = $"UD_BIN_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col BINARY)";
        createCmd.ExecuteNonQuery();

        // And The table is populated with binary values [X'48656C6C6F', X'576F726C64', X'0123456789ABCDEF']
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (X'48656C6C6F'), (X'576F726C64'), (X'0123456789ABCDEF')";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM {table} ORDER BY col" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then the result should contain binary values in order:
        Assert.True(reader.Read(), "Expected row 0");
        Assert.Equal(new byte[] { 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF }, (byte[])reader.GetValue(0));
        Assert.True(reader.Read(), "Expected row 1");
        Assert.Equal("Hello"u8.ToArray(), (byte[])reader.GetValue(0));
        Assert.True(reader.Read(), "Expected row 2");
        Assert.Equal("World"u8.ToArray(), (byte[])reader.GetValue(0));
        Assert.False(reader.Read(), "Expected exactly 3 rows");
    }

    // Scenario: should select corner case binary values from table
    [SnowflakeFact]
    public void ShouldSelectCornerCaseBinaryValuesFromTable()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And A temporary table with BINARY column is created
        var tableName = $"UD_BIN_CRN_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col BINARY)";
        createCmd.ExecuteNonQuery();

        // And The table is populated with corner case binary values
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (X''), (X'00'), (X'FF'), (X'000000'), (X'48006500')";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM {table} ORDER BY 1" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY 1";
        using var reader = selectCmd.ExecuteReader();

        // Then the result should contain the inserted corner case binary values
        var rows = new List<byte[]>();
        while (reader.Read())
        {
            rows.Add((byte[])reader.GetValue(0));
        }
        Assert.Equal(5, rows.Count);
        Assert.Contains(rows, r => r.Length == 0);
        Assert.Contains(rows, r => r.Length == 1 && r[0] == 0x00);
        Assert.Contains(rows, r => r.Length == 1 && r[0] == 0xFF);
        Assert.Contains(rows, r => r.Length == 3 && r.All(b => b == 0x00));
        Assert.Contains(rows, r => r.SequenceEqual("H\0e\0"u8));
    }

    // Scenario: should select NULL binary values from table
    [SnowflakeFact]
    public void ShouldSelectNullBinaryValuesFromTable()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And A temporary table with BINARY column is created
        var tableName = $"UD_BIN_NUL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col BINARY)";
        createCmd.ExecuteNonQuery();

        // And The table is populated with NULL and non-NULL binary values [NULL, X'ABCD', NULL]
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (NULL), (X'ABCD'), (NULL)";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = selectCmd.ExecuteReader();

        // Then there are 3 rows returned
        var nullCount = 0;
        var nonNullValues = new List<byte[]>();
        while (reader.Read())
        {
            if (reader.IsDBNull(0))
                nullCount++;
            else
                nonNullValues.Add((byte[])reader.GetValue(0));
        }

        // And 2 rows should contain NULL values
        Assert.Equal(2, nullCount);

        // And 1 row should contain 0xABCD
        Assert.Single(nonNullValues);
        Assert.Equal(new byte[] { 0xAB, 0xCD }, nonNullValues[0]);
    }

    // Scenario: should select binary with specified length from table
    [SnowflakeFact]
    public void ShouldSelectBinaryWithSpecifiedLengthFromTable()
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with columns (bin5 BINARY(5), bin10 BINARY(10), bin_default BINARY) exists
        var tableName = $"UD_BIN_LEN_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (bin5 BINARY(5), bin10 BINARY(10), bin_default BINARY)";
        createCmd.ExecuteNonQuery();

        // And Row (X'0102030405', X'01020304050607080910', X'48656C6C6F') is inserted
        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} VALUES (X'0102030405', X'01020304050607080910', X'48656C6C6F')";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain binary values with correct lengths
        Assert.True(reader.Read(), "Expected one row");
        var bin5 = (byte[])reader.GetValue(0);
        var bin10 = (byte[])reader.GetValue(1);
        var binDefault = (byte[])reader.GetValue(2);
        Assert.Equal(5, bin5.Length);
        Assert.Equal(new byte[] { 0x01, 0x02, 0x03, 0x04, 0x05 }, bin5);
        Assert.Equal(10, bin10.Length);
        Assert.Equal(new byte[] { 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x10 }, bin10);
        Assert.Equal("Hello"u8.ToArray(), binDefault);
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should select binary literals using parameter binding
    [SnowflakeFact]
    public void ShouldSelectBinaryLiteralsUsingParameterBinding()
    {
        Skip.FutureMilestone();
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::BINARY, ?::BINARY, ?::BINARY" is executed with bound binary values [0x48656C6C6F, 0x576F726C64, 0x0123456789ABCDEF]
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT ?::BINARY, ?::BINARY, ?::BINARY";

        var param1 = cmd.CreateParameter();
        param1.ParameterName = "1";
        param1.DbType = DbType.Binary;
        param1.Value = "Hello"u8.ToArray();
        cmd.Parameters.Add(param1);

        var param2 = cmd.CreateParameter();
        param2.ParameterName = "2";
        param2.DbType = DbType.Binary;
        param2.Value = "World"u8.ToArray();
        cmd.Parameters.Add(param2);

        var param3 = cmd.CreateParameter();
        param3.ParameterName = "3";
        param3.DbType = DbType.Binary;
        param3.Value = new byte[] { 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF };
        cmd.Parameters.Add(param3);

        using var reader = cmd.ExecuteReader();

        // Then the result should contain:
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal("Hello"u8.ToArray(), (byte[])reader.GetValue(0));
        Assert.Equal("World"u8.ToArray(), (byte[])reader.GetValue(1));
        Assert.Equal(new byte[] { 0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF }, (byte[])reader.GetValue(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should insert binary using parameter binding
    [SnowflakeFact]
    public void ShouldInsertBinaryUsingParameterBinding()
    {
        Skip.FutureMilestone();
        var insertValues = new[]
        {
            "Hello"u8.ToArray(),
            "World"u8.ToArray(),
            [0x00],
            [0xFF],
            []
        };

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with BINARY column exists
        var tableName = $"UD_BIN_INS_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col BINARY)";
        createCmd.ExecuteNonQuery();

        // When Binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x] are inserted using binding
        foreach (var value in insertValues)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.Binary;
            param.Value = value;
            insertCmd.Parameters.Add(param);
            insertCmd.ExecuteNonQuery();
        }

        // And Query "SELECT * FROM {table}" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain binary values [0x48656C6C6F, 0x576F726C64, 0x00, 0xFF, 0x]
        var results = new List<byte[]>();
        while (reader.Read())
        {
            results.Add((byte[])reader.GetValue(0));
        }
        Assert.Equal(insertValues.Length, results.Count);
        foreach (var expected in insertValues)
        {
            Assert.Contains(results, r => r.SequenceEqual(expected));
        }
    }

    // Scenario: should bind corner case binary values
    [SnowflakeFact]
    public void ShouldBindCornerCaseBinaryValues()
    {
        Skip.FutureMilestone();
        var cornerCases = new (byte[]? value, bool isNull)[]
        {
            ([], false),
            ([0x00], false),
            ([0xFF], false),
            ("H\0e\0"u8.ToArray(), false),
            (null, true)
        };

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::BINARY" is executed with each corner case binary value bound
        foreach (var (value, isNull) in cornerCases)
        {
            using var cmd = connection.CreateCommand();
            cmd.CommandText = "SELECT ?::BINARY";
            var param = cmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.Binary;
            param.Value = isNull ? (object)DBNull.Value : value!;
            cmd.Parameters.Add(param);
            using var reader = cmd.ExecuteReader();

            // Then the result should match the bound corner case value
            Assert.True(reader.Read(), "Expected one row");
            if (isNull)
            {
                Assert.True(reader.IsDBNull(0), "Expected NULL");
            }
            else
            {
                Assert.False(reader.IsDBNull(0), "Expected non-NULL");
                Assert.Equal(value, (byte[])reader.GetValue(0));
            }
            Assert.False(reader.Read(), "Expected exactly one row");
        }
    }

    // Scenario: should download binary data in multiple chunks using GENERATOR
    [SnowflakeFact]
    public void ShouldDownloadBinaryDataInMultipleChunksUsingGenerator()
    {
        const int rowCount = 30000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT seq8() AS id, TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8') AS bin_val FROM TABLE(GENERATOR(ROWCOUNT => 30000)) v ORDER BY id" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT seq8() AS id, TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8') AS bin_val FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) v ORDER BY id";
        using var reader = cmd.ExecuteReader();

        // Then there are 30000 rows returned
        var count = 0;
        while (reader.Read())
        {
            var id = reader.GetInt64(0);
            var binVal = (byte[])reader.GetValue(1);

            // And all returned binary values should match the generated values in order
            var expectedStr = id.ToString().PadLeft(10, '0');
            var expectedBytes = System.Text.Encoding.UTF8.GetBytes(expectedStr);
            Assert.Equal(expectedBytes, binVal);
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should download binary data in multiple chunks from table
    [SnowflakeFact]
    public void ShouldDownloadBinaryDataInMultipleChunksFromTable()
    {
        const int rowCount = 30000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with (bin_data BINARY) exists with 30000 sequential binary values
        var tableName = $"UD_BIN_CHK_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (bin_data BINARY)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} SELECT TO_BINARY(LPAD(TO_VARCHAR(seq8()), 10, '0'), 'UTF-8') FROM TABLE(GENERATOR(ROWCOUNT => {rowCount}))";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM {table} ORDER BY bin_data" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY bin_data";
        using var reader = selectCmd.ExecuteReader();

        // Then there are 30000 rows returned
        var count = 0;
        while (reader.Read())
        {
            var binVal = (byte[])reader.GetValue(0);

            // And all returned binary values should match the inserted values in order
            var expectedStr = count.ToString().PadLeft(10, '0');
            var expectedBytes = System.Text.Encoding.UTF8.GetBytes(expectedStr);
            Assert.Equal(expectedBytes, binVal);
            count++;
        }
        Assert.Equal(rowCount, count);
    }
}
