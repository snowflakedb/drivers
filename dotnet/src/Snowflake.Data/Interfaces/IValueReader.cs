using Snowflake.Data.Core;

namespace Snowflake.Data.Interfaces;

/// <summary>
/// Reads a column's values as <typeparamref name="T"/>.
/// </summary>
/// <typeparam name="T">The CTS type the value is read as.</typeparam>
internal interface IValueReader<out T>
{
    /// <summary>
    /// The value at <paramref name="row"/> as <typeparamref name="T"/>. Throws
    /// <see cref="SFError.INVALID_DATA_CONVERSION"/> for a value that cannot be represented, including a NULL
    /// value unless the converter documents otherwise.
    /// </summary>
    T? Read(int row);
}
