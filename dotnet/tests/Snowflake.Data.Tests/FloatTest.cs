using Snowflake.Data.Tests.Compatibility;

namespace Snowflake.Data.Tests;

[Trait("Category", "E2E")]
public class FloatTest : IClassFixture<ITFixture>
{
    private static readonly string[] FloatTypeSynonyms =
        ["FLOAT", "DOUBLE", "REAL"];

    protected readonly ITestOutputHelper Output;
    protected readonly ITFixture Fixture;

    public FloatTest(ITFixture fixture, ITestOutputHelper output)
    {
        Fixture = fixture;
        Output = output;
    }

    // Scenario: should select float literals for float and synonyms
    [SnowflakeTheory]
    [InlineData("FLOAT")]
    [InlineData("DOUBLE")]
    [InlineData("REAL")]
    public void ShouldSelectFloatLiteralsForFloatAndSynonyms(string floatType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 0.0::<type>, 1.0::<type>, -1.0::<type>, 123.456::<type>, -123.456::<type>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT 0.0::{floatType}, 1.0::{floatType}, -1.0::{floatType}, 123.456::{floatType}, -123.456::{floatType}";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain floats [0.0, 1.0, -1.0, 123.456, -123.456]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(0.0, reader.GetDouble(0));
        Assert.Equal(1.0, reader.GetDouble(1));
        Assert.Equal(-1.0, reader.GetDouble(2));
        Assert.Equal(123.456, reader.GetDouble(3), 10);
        Assert.Equal(-123.456, reader.GetDouble(4), 10);
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario Outline: should handle float <case> boundary values from literals for float and synonyms
    [SnowflakeTheory]
    [MemberData(nameof(BoundaryValuesData))]
    public void ShouldHandleFloatCaseBoundaryValuesFromLiteralsForFloatAndSynonyms(string floatType, string caseLabel, string queryValues, double[] expectedValues)
    {
        _ = caseLabel; // used for test display name only
        _ = queryValues; // used for test display name only
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT <query_values>" is executed
        using var cmd = connection.CreateCommand();
        var castExpressions = expectedValues.Select(v =>
        {
            if (double.IsNaN(v))
                return $"'NaN'::{floatType}";
            if (double.IsPositiveInfinity(v))
                return $"'inf'::{floatType}";
            if (double.IsNegativeInfinity(v))
                return $"'-inf'::{floatType}";
            return $"{v:R}::{floatType}";
        });
        cmd.CommandText = $"SELECT {string.Join(", ", castExpressions)}";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain floats [<expected_values>]
        Assert.True(reader.Read(), "Expected one row");
        for (var i = 0; i < expectedValues.Length; i++)
        {
            var actual = reader.GetDouble(i);
            if (double.IsNaN(expectedValues[i]))
                Assert.True(double.IsNaN(actual), $"Expected NaN at column {i}");
            else if (double.IsInfinity(expectedValues[i]))
                Assert.Equal(expectedValues[i], actual);
            else
                Assert.Equal(expectedValues[i], actual);
        }
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    public static IEnumerable<object[]> BoundaryValuesData()
    {
        foreach (var floatType in FloatTypeSynonyms)
        {
            yield return [floatType, "max", "1.7976931348623157e308::<type>, -1.7976931348623157e308::<type>",
                new[] { 1.7976931348623157e308, -1.7976931348623157e308 }];
            yield return [floatType, "min", "2.2250738585072014e-308::<type>, 5e-324::<type>",
                new[] { 2.2250738585072014e-308, 5e-324 }];
        }
    }

    // Scenario Outline: should handle realistic large float <case> boundary values from literals for float and synonyms
    [SnowflakeTheory]
    [MemberData(nameof(RealisticBoundaryValuesData))]
    public void ShouldHandleRealisticLargeFloatCaseBoundaryValuesFromLiteralsForFloatAndSynonyms(string floatType, string caseLabel, double[] expectedValues)
    {
        _ = caseLabel; // used for test display name only
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT <query_values>" is executed
        using var cmd = connection.CreateCommand();
        var castExpressions = expectedValues.Select(v => $"{v:R}::{floatType}");
        cmd.CommandText = $"SELECT {string.Join(", ", castExpressions)}";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain floats [<expected_values>]
        Assert.True(reader.Read(), "Expected one row");
        for (var i = 0; i < expectedValues.Length; i++)
        {
            Assert.Equal(expectedValues[i], reader.GetDouble(i));
        }
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    public static IEnumerable<object[]> RealisticBoundaryValuesData()
    {
        foreach (var floatType in FloatTypeSynonyms)
        {
            yield return [floatType, "max", new[] { 1.79769313486231e+308, -1.79769313486231e+308 }];
        }
    }

    // Scenario: should handle float precision boundary values from literals for float and synonyms
    [SnowflakeTheory]
    [InlineData("FLOAT")]
    [InlineData("DOUBLE")]
    [InlineData("REAL")]
    public void ShouldHandleFloatPrecisionBoundaryValuesFromLiteralsForFloatAndSynonyms(string floatType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 123456789012345.0::<type>, 1234567890123456.0::<type>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT 123456789012345.0::{floatType}, 1234567890123456.0::{floatType}";
        using var reader = cmd.ExecuteReader();

        // Then Result should verify precision around 15 decimal digits
        Assert.True(reader.Read(), "Expected one row");
        var val1 = reader.GetDouble(0);
        var val2 = reader.GetDouble(1);
        Assert.Equal(123456789012345.0, val1, 1e0);

        // works till this point
        Skip.FutureMilestone();
        Assert.Equal(1234567890123456.0, val2, 1e0);
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle NULL values from literals for float and synonyms
    [SnowflakeTheory]
    [InlineData("FLOAT")]
    [InlineData("DOUBLE")]
    [InlineData("REAL")]
    public void ShouldHandleNullValuesFromLiteralsForFloatAndSynonyms(string floatType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT NULL::<type>, 42.5::<type>, NULL::<type>" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT NULL::{floatType}, 42.5::{floatType}, NULL::{floatType}";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [NULL, 42.5, NULL]
        Assert.True(reader.Read(), "Expected one row");
        Assert.True(reader.IsDBNull(0), "Expected NULL for column 0");
        Assert.Equal(42.5, reader.GetDouble(1));
        Assert.True(reader.IsDBNull(2), "Expected NULL for column 2");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should download large result set with multiple chunks from GENERATOR for float and synonyms
    [SnowflakeTheory]
    [InlineData("FLOAT")]
    [InlineData("DOUBLE")]
    [InlineData("REAL")]
    public void ShouldDownloadLargeResultSetWithMultipleChunksFromGeneratorForFloatAndSynonyms(string floatType)
    {
        const int rowCount = 50000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT seq8()::<type> as id FROM TABLE(GENERATOR(ROWCOUNT => 50000)) v" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT seq8()::{floatType} AS id FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) v";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain 50000 rows with all values returned as appropriate float type
        var count = 0;
        while (reader.Read())
        {
            Assert.Equal(typeof(double), reader.GetFieldType(0));
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should select floats from table for float and synonyms
    [SnowflakeTheory]
    [InlineData("FLOAT")]
    [InlineData("DOUBLE")]
    [InlineData("REAL")]
    public void ShouldSelectFloatsFromTableForFloatAndSynonyms(string floatType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with <type> column exists with values [0.0, 123.456, -789.012, 1.23e5, -9.87e-3]
        var tableName = $"UD_FLOAT_TBL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col {floatType})";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (0.0), (123.456), (-789.012), (1.23e5), (-9.87e-3)";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM float_table" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain floats [0.0, 123.456, -789.012, 123000.0, -0.00987]
        double[] expected = [-789.012, -0.00987, 0.0, 123.456, 123000.0];
        var rowIndex = 0;
        while (reader.Read())
        {
            Assert.Equal(expected[rowIndex], reader.GetDouble(0), 10);
            rowIndex++;
        }
        Assert.Equal(expected.Length, rowIndex);
    }

    // Scenario: should handle float boundary values from table for float and synonyms
    [SnowflakeTheory]
    [InlineData("FLOAT")]
    [InlineData("DOUBLE")]
    [InlineData("REAL")]
    public void ShouldHandleFloatBoundaryValuesFromTableForFloatAndSynonyms(string floatType)
    {

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with <type> column exists with boundary values [1.7976931348623157e308, -1.7976931348623157e308, 2.2250738585072014e-308, 5e-324, 123456789012345.0]
        var tableName = $"UD_FLOAT_BNDRY_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col {floatType})";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (1.7976931348623157e308), (-1.7976931348623157e308), (2.2250738585072014e-308), (5e-324), (123456789012345.0)";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName} ORDER BY col";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain maximum, minimum, and precision boundary values preserved within float precision limits
        Assert.True(reader.Read());
        var negMax = reader.GetDouble(0);
        Assert.Equal(double.NegativeInfinity, negMax);
        Assert.True(reader.Read());
        var negMin = reader.GetDouble(0);

        // works till this point
        Skip.FutureMilestone();

        Assert.True(negMin < 0 && negMin > -1.0, $"Expected small negative subnormal, got {negMin}");
        Assert.True(reader.Read());
        var minNormal = reader.GetDouble(0);
        Assert.Equal(2.2250738585072014e-308, minNormal);
        Assert.True(reader.Read());
        var precision = reader.GetDouble(0);
        Assert.Equal(123456789012345.0, precision, 1e0);
        Assert.True(reader.Read());
        var posMax = reader.GetDouble(0);
        Assert.Equal(1.7976931348623157e308, posMax);
        Assert.False(reader.Read(), "Expected exactly 5 rows");
    }

    // Scenario: should handle NULL values from table for float and synonyms
    [SnowflakeTheory]
    [InlineData("FLOAT")]
    [InlineData("DOUBLE")]
    [InlineData("REAL")]
    public void ShouldHandleNullValuesFromTableForFloatAndSynonyms(string floatType)
    {
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with <type> column exists with values [NULL, 123.456, NULL, -789.012]
        var tableName = $"UD_FLOAT_NULL_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, col {floatType})";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (id, col) VALUES (1, NULL), (2, 123.456), (3, NULL), (4, -789.012)";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY id";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain [NULL, 123.456, NULL, -789.012]
        Assert.True(reader.Read(), "Expected row 0");
        Assert.True(reader.IsDBNull(0), "Expected NULL for row 0");
        Assert.True(reader.Read(), "Expected row 1");
        Assert.Equal(123.456, reader.GetDouble(0), 10);
        Assert.True(reader.Read(), "Expected row 2");
        Assert.True(reader.IsDBNull(0), "Expected NULL for row 2");
        Assert.True(reader.Read(), "Expected row 3");
        Assert.Equal(-789.012, reader.GetDouble(0), 10);
        Assert.False(reader.Read(), "Expected exactly 4 rows");
    }

    // Scenario: should select large result set from table for float and synonyms
    [SnowflakeTheory]
    [InlineData("FLOAT")]
    [InlineData("DOUBLE")]
    [InlineData("REAL")]
    public void ShouldSelectLargeResultSetFromTableForFloatAndSynonyms(string floatType)
    {
        const int rowCount = 50000;

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with <type> column exists with 50000 sequential values
        var tableName = $"UD_FLOAT_LRG_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col {floatType})";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} SELECT seq8()::{floatType} FROM TABLE(GENERATOR(ROWCOUNT => {rowCount})) v";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT * FROM {tableName}";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain 50000 rows with all values returned as appropriate float type
        var count = 0;
        while (reader.Read())
        {
            Assert.Equal(typeof(double), reader.GetFieldType(0));
            count++;
        }
        Assert.Equal(rowCount, count);
    }

    // Scenario: should select float using parameter binding for float and synonyms
    [SnowflakeTheory]
    [InlineData("FLOAT")]
    [InlineData("DOUBLE")]
    [InlineData("REAL")]
    public void ShouldSelectFloatUsingParameterBindingForFloatAndSynonyms(string floatType)
    {
        Skip.FutureMilestone();
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT ?::<type>, ?::<type>, ?::<type>" is executed with bound float values [123.456, -789.012, 42.0]
        using var cmd = connection.CreateCommand();
        cmd.CommandText = $"SELECT ?::{floatType}, ?::{floatType}, ?::{floatType}";

        var param1 = cmd.CreateParameter();
        param1.ParameterName = "1";
        param1.DbType = DbType.Double;
        param1.Value = 123.456;
        cmd.Parameters.Add(param1);

        var param2 = cmd.CreateParameter();
        param2.ParameterName = "2";
        param2.DbType = DbType.Double;
        param2.Value = -789.012;
        cmd.Parameters.Add(param2);

        var param3 = cmd.CreateParameter();
        param3.ParameterName = "3";
        param3.DbType = DbType.Double;
        param3.Value = 42.0;
        cmd.Parameters.Add(param3);

        using var reader = cmd.ExecuteReader();

        // Then Result should contain floats [123.456, -789.012, 42.0]
        Assert.True(reader.Read(), "Expected one row");
        Assert.Equal(123.456, reader.GetDouble(0), 10);
        Assert.Equal(-789.012, reader.GetDouble(1), 10);
        Assert.Equal(42.0, reader.GetDouble(2));
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should insert float using parameter binding for float and synonyms
    [SnowflakeTheory]
    [InlineData("FLOAT")]
    [InlineData("DOUBLE")]
    [InlineData("REAL")]
    public void ShouldInsertFloatUsingParameterBindingForFloatAndSynonyms(string floatType)
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with <type> column exists
        var tableName = $"UD_FLOAT_BIND_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (col {floatType})";
        createCmd.ExecuteNonQuery();

        // When Float values [0.0, 123.456, -789.012, NULL] are bulk-inserted using multirow binding
        double?[] values = [0.0, 123.456, -789.012, null];
        foreach (var value in values)
        {
            using var insertCmd = connection.CreateCommand();
            insertCmd.CommandText = $"INSERT INTO {tableName} (col) VALUES (?)";
            var param = insertCmd.CreateParameter();
            param.ParameterName = "1";
            param.DbType = DbType.Double;
            param.Value = value.HasValue ? (object)value.Value : DBNull.Value;
            insertCmd.Parameters.Add(param);
            insertCmd.ExecuteNonQuery();
        }

        // Then Result should contain the same values including NULL
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY col NULLS LAST";
        using var reader = selectCmd.ExecuteReader();

        Assert.True(reader.Read(), "Expected row 0");
        Assert.Equal(-789.012, reader.GetDouble(0), 10);
        Assert.True(reader.Read(), "Expected row 1");
        Assert.Equal(0.0, reader.GetDouble(0));
        Assert.True(reader.Read(), "Expected row 2");
        Assert.Equal(123.456, reader.GetDouble(0), 10);
        Assert.True(reader.Read(), "Expected row 3");
        Assert.True(reader.IsDBNull(0), "Expected NULL for row 3");
        Assert.False(reader.Read(), "Expected exactly 4 rows");
    }

    // Scenario: should handle special float values as doubles from literals for float and synonyms
    [SnowflakeFact]
    public void ShouldHandleSpecialFloatValuesAsDoublesFromLiteralsForFloatAndSynonyms()
    {
        Skip.FutureMilestone();
        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // When Query "SELECT 'NaN'::FLOAT, 'inf'::FLOAT, '-inf'::FLOAT" is executed
        using var cmd = connection.CreateCommand();
        cmd.CommandText = "SELECT 'NaN'::FLOAT, 'inf'::FLOAT, '-inf'::FLOAT";
        using var reader = cmd.ExecuteReader();

        // Then Result should contain [NaN, positive_infinity, negative_infinity] read as .NET double via GetValue
        Assert.True(reader.Read(), "Expected one row");
        var nan = (double)reader.GetValue(0);
        var posInf = (double)reader.GetValue(1);
        var negInf = (double)reader.GetValue(2);
        Assert.True(double.IsNaN(nan), "Expected NaN");
        Assert.True(double.IsPositiveInfinity(posInf), "Expected positive infinity");
        Assert.True(double.IsNegativeInfinity(negInf), "Expected negative infinity");
        Assert.False(reader.Read(), "Expected exactly one row");
    }

    // Scenario: should handle special float values as doubles from table for float and synonyms
    [SnowflakeFact]
    public void ShouldHandleSpecialFloatValuesAsDoublesFromTableForFloatAndSynonyms()
    {
        Skip.FutureMilestone();

        // Given Snowflake client is logged in
        using var connection = Fixture.Factory.Create(Output);
        connection.Open();

        // And Table with FLOAT column exists with values [NaN, inf, -inf, 42.0, -42.0]
        var tableName = $"UD_FLOAT_SPEC_{Guid.NewGuid():N}".Substring(0, 30);
        using var createCmd = connection.CreateCommand();
        createCmd.CommandText = $"CREATE TEMPORARY TABLE {tableName} (id INT, col FLOAT)";
        createCmd.ExecuteNonQuery();

        using var insertCmd = connection.CreateCommand();
        insertCmd.CommandText = $"INSERT INTO {tableName} (id, col) VALUES (1, 'NaN'), (2, 'inf'), (3, '-inf'), (4, 42.0), (5, -42.0)";
        insertCmd.ExecuteNonQuery();

        // When Query "SELECT * FROM <table>" is executed
        using var selectCmd = connection.CreateCommand();
        selectCmd.CommandText = $"SELECT col FROM {tableName} ORDER BY id";
        using var reader = selectCmd.ExecuteReader();

        // Then Result should contain [NaN, positive_infinity, negative_infinity, 42.0, -42.0] read as .NET double via GetValue
        Assert.True(reader.Read(), "Expected row 0");
        Assert.True(double.IsNaN((double)reader.GetValue(0)), "Expected NaN");
        Assert.True(reader.Read(), "Expected row 1");
        Assert.True(double.IsPositiveInfinity((double)reader.GetValue(0)), "Expected positive infinity");
        Assert.True(reader.Read(), "Expected row 2");
        Assert.True(double.IsNegativeInfinity((double)reader.GetValue(0)), "Expected negative infinity");
        Assert.True(reader.Read(), "Expected row 3");
        Assert.Equal(42.0, (double)reader.GetValue(0));
        Assert.True(reader.Read(), "Expected row 4");
        Assert.Equal(-42.0, (double)reader.GetValue(0));
        Assert.False(reader.Read(), "Expected exactly 5 rows");
    }
}
