using System.Globalization;
using System.Resources;
using Snowflake.Data.Attributes;
using Snowflake.Data.Compatibility;

namespace Snowflake.Data.Core.Extensions;

internal static class SFErrorExtensions
{
    private static readonly ResourceManager MessageTemplates = new("Snowflake.Data.Core.ErrorMessages", typeof(SFErrorExtensions).Assembly);

    private static IReadOnlyDictionary<SFError, int> ErrorToErrorCodeMap { get; } = InitErrorToErrorCodeMap();

    public static int GetErrorCode(this SFError error) => ErrorToErrorCodeMap[error];

    /// <summary>Formats the message template of <paramref name="error"/> with <paramref name="args"/>.</summary>
    /// <remarks>
    /// An error without a template, such as a server vendor code the driver names, gives its own name.
    /// </remarks>
    /// <exception cref="FormatException">The template has more placeholders than <paramref name="args"/> holds.</exception>
    public static string FormatMessage(this SFError error, params object[] args)
    {
        var template = MessageTemplates.GetString(error.ToString(), CultureInfo.InvariantCulture) ?? error.ToString();
        return string.Format(CultureInfo.InvariantCulture, template, args);
    }

    private static Dictionary<SFError, int> InitErrorToErrorCodeMap()
    {
        return EnumHelper
            .GetValues<SFError>()
            .Select(x => (x, typeof(SFError).GetMember(x.ToString())))
            .Select(x => (x.x, (SFErrorAttribute)x.Item2[0].GetCustomAttributes(typeof(SFErrorAttribute), false).Single()))
            .ToDictionary(x => x.x, x => x.Item2.ErrorCode);
    }
}
