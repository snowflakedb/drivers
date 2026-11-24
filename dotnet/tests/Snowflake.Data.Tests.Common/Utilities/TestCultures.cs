using System.Globalization;

namespace Snowflake.Data.Tests.Utilities;

/// <summary>
/// Cultures whose formatting differs from the invariant culture on every target framework and OS.
/// </summary>
/// <remarks>
/// Named cultures are not used for this because their data depends on the platform: sv-SE has a U+2212 MINUS SIGN
/// as its negative sign under ICU, but a hyphen-minus under the Windows NLS data that .NET Framework reads.
/// </remarks>
public static class TestCultures
{
    /// <summary>Returns the invariant culture with U+2212 MINUS SIGN as its negative sign.</summary>
    public static CultureInfo UnicodeMinus()
    {
        var culture = (CultureInfo)CultureInfo.InvariantCulture.Clone();
        culture.NumberFormat.NegativeSign = "\u2212";
        return culture;
    }
}
