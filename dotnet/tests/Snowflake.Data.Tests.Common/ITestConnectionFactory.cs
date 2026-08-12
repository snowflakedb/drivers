using System.Data.Common;

namespace Snowflake.Data.Tests;

public interface ITestConnectionFactory
{
    DbConnection Create(ITestOutputHelper? testOutputHelper, string? connectionStringOverride = null);
}
