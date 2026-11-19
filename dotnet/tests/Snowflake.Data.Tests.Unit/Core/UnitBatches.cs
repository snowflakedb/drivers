using System.Numerics;
using Apache.Arrow;
using Apache.Arrow.Types;
using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Core;

/// <summary>
/// Arrow arrays, batches and descriptors in the shapes sf_core produces, for unit tests.
/// </summary>
internal static class UnitBatches
{
    internal static ColumnMetadata FixedColumn(string name = "C1", long scale = 0) =>
        new() { Name = name, Type = "fixed", Scale = scale, Precision = 38, Nullable = true };

    internal static ColumnMetadata TextColumn(string name = "C1") =>
        new() { Name = name, Type = "text", Nullable = true };

    internal static ColumnMetadata Column(string name, string type) =>
        new() { Name = name, Type = type, Nullable = true };

    internal static ResultSetDescriptor Descriptor(params ColumnMetadata[] columns)
    {
        var descriptor = new ResultSetDescriptor { QueryId = "01bf4a3e-0001-2c4f-0000-00a1b2c3d4e5" };
        descriptor.Columns.Add(columns);
        return descriptor;
    }

    internal static Int64Array Int64(params long?[] values)
    {
        var builder = new Int64Array.Builder();
        foreach (var value in values)
        {
            if (value is { } present)
                builder.Append(present);
            else
                builder.AppendNull();
        }

        return builder.Build();
    }

    internal static StringArray Strings(params string?[] values)
    {
        var builder = new StringArray.Builder();
        foreach (var value in values)
        {
            if (value is null)
                builder.AppendNull();
            else
                builder.Append(value);
        }

        return builder.Build();
    }

    internal static Decimal128Array Decimals(params decimal?[] values)
    {
        var builder = new Decimal128Array.Builder(new Decimal128Type(38, 0));
        foreach (var value in values)
        {
            if (value is { } present)
                builder.Append(present);
            else
                builder.AppendNull();
        }

        return builder.Build();
    }

    /// <summary>A NUMBER(38,0) array holding <paramref name="values"/>, which may lie outside the <see cref="decimal"/> range.</summary>
    internal static Decimal128Array Int128(params BigInteger[] values)
    {
        var bytes = new byte[16 * values.Length];
        for (var i = 0; i < values.Length; i++)
        {
            var value = values[i].ToByteArray();
            var signExtension = values[i].Sign < 0 ? (byte)0xFF : (byte)0x00;
            for (var b = 0; b < 16; b++)
                bytes[(16 * i) + b] = b < value.Length ? value[b] : signExtension;
        }

        var data = new ArrayData(new Decimal128Type(38, 0), values.Length, 0, 0, [ArrowBuffer.Empty, new ArrowBuffer(bytes)]);
        return new Decimal128Array(data);
    }

    /// <summary>A batch whose columns are <paramref name="columns"/>, named C1, C2, ….</summary>
    internal static RecordBatch Batch(params IArrowArray[] columns)
    {
        var fields = columns.Select((column, i) => new Field($"C{i + 1}", column.Data.DataType, nullable: true));
        return new RecordBatch(new Schema(fields, null), columns, columns.Length == 0 ? 0 : columns[0].Length);
    }
}
