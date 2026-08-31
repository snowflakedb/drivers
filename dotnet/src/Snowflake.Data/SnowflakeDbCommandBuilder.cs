using System.Data;
using System.Data.Common;
// ReSharper disable UnusedType.Global
// ReSharper disable UnusedParameter.Local
#pragma warning disable CS8765 // Nullability of type of parameter doesn't match overridden member (possibly because of nullability attributes).

namespace Snowflake.Data;

public class SnowflakeDbCommandBuilder : DbCommandBuilder
{
    public SnowflakeDbCommandBuilder()
        : this(null)
    {
    }

    public SnowflakeDbCommandBuilder(SnowflakeDbDataAdapter? adapter)
    {
        throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
    }

    public override string QuotePrefix
    {
        get => throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
        set => throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
    }

    public override string QuoteSuffix
    {
        get => throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
        set => throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
    }

    protected override void ApplyParameterInfo(DbParameter p, DataRow row, StatementType statementType, bool whereClause) =>
        throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    protected override string GetParameterName(int parameterOrdinal) => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    protected override string GetParameterName(string parameterName) => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    protected override string GetParameterPlaceholder(int parameterOrdinal) => throw new NotImplementedException(
        "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");

    protected override void SetRowUpdatingHandler(DbDataAdapter adapter) =>
        throw new NotImplementedException(
            "TODO this implementation is a stub for now. It awaits implementation (or dropping if applicable)");
}
