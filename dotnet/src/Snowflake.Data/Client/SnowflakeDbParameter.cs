using System.Data;
using System.Data.Common;
using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using Snowflake.Data.Core;
using Snowflake.Data.Core.DataConversion.ParameterConversion;

namespace Snowflake.Data.Client;

public sealed class SnowflakeDbParameter : DbParameter
{
    private readonly SFDataType _originalSFDataType;

    public SnowflakeDbParameter()
    {
    }

    /// <summary>Creates a parameter that binds as <paramref name="sfDataType"/>, which <see cref="ResetDbType"/> restores.</summary>
    public SnowflakeDbParameter(string parameterName, SFDataType sfDataType)
    {
        ParameterName = parameterName;
        SFDataType = sfDataType;
        _originalSFDataType = sfDataType;
    }

    /// <summary>
    /// Creates a parameter for the <paramref name="parameterIndex"/>-th positional placeholder that binds as
    /// <paramref name="sfDataType"/>. <see cref="ResetDbType"/> resets its type to <see cref="Core.SFDataType.None"/>.
    /// </summary>
    public SnowflakeDbParameter(int parameterIndex, SFDataType sfDataType)
    {
        ParameterName = parameterIndex.ToString(CultureInfo.InvariantCulture);
        SFDataType = sfDataType;
    }

    /// <summary>
    /// Gets or sets the Snowflake type the value binds as. Any value other than <see cref="Core.SFDataType.None"/>
    /// takes precedence over <see cref="DbType"/>.
    /// </summary>
    public SFDataType SFDataType { get; set; }

    /// <summary>
    /// Gets or sets the type the value binds as. Until it is set to anything other than <see cref="DbType.AnsiString"/>,
    /// it is inferred from the CLR type of a non-null <see cref="Value"/>, or of its elements when the value is an
    /// array other than <c>byte[]</c> or <c>char[]</c>; a CLR type with no counterpart infers <see cref="DbType.Object"/>.
    /// </summary>
    public override DbType DbType
    {
        get => field != default || Value is null or DBNull ? field : ParameterBindingResolver.InferDbType(Value);
        set;
    }

    // TODO(SNOW-4382591): a direction other than Input is bound as input instead of throwing UNSUPPORTED_FEATURE.
    public override ParameterDirection Direction { get; set; } = ParameterDirection.Input;

    public override bool IsNullable { get; set; }

    [AllowNull]
    public override string ParameterName { get; set => field = value ?? string.Empty; } = string.Empty;

    public override int Size { get; set; }

    [AllowNull]
    public override string SourceColumn { get; set; } = string.Empty;

    public override bool SourceColumnNullMapping { get; set; }

    public override object? Value { get; set; }

    /// <summary>
    /// Restores <see cref="SFDataType"/> to the type the parameter was created with; <see cref="DbType"/> is kept.
    /// </summary>
    public override void ResetDbType() => SFDataType = _originalSFDataType;
}
