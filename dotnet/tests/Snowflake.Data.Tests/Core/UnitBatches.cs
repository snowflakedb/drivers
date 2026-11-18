using Apache.Arrow;

namespace Snowflake.Data.Tests.Core;

/// <summary>
/// Arrow arrays, batches and descriptors in the shapes sf_core produces, for unit tests.
/// </summary>
internal static class UnitBatches
{
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

    /// <summary>A batch whose columns are <paramref name="columns"/>, named C1, C2, ….</summary>
    internal static RecordBatch Batch(params IArrowArray[] columns)
    {
        var fields = columns.Select((column, i) => new Field($"C{i + 1}", column.Data.DataType, nullable: true));
        return new RecordBatch(new Schema(fields, null), columns, columns.Length == 0 ? 0 : columns[0].Length);
    }
}
