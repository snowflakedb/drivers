using Snowflake.Data.Attributes;
using Snowflake.Data.Compatibility;

namespace Snowflake.Data.Core.Extensions;

internal static class SFErrorExtensions
{
    private static IReadOnlyDictionary<SFError, int> ErrorToErrorCodeMap { get; } = InitErrorToErrorCodeMap();

    public static int GetErrorCode(this SFError error) => ErrorToErrorCodeMap[error];

    private static Dictionary<SFError, int> InitErrorToErrorCodeMap()
    {
        return EnumHelper
            .GetValues<SFError>()
            .Select(x => (x, typeof(SFError).GetMember(x.ToString())))
            .Select(x => (x.x, (SFErrorAttribute)x.Item2[0].GetCustomAttributes(typeof(SFErrorAttribute), false).Single()))
            .ToDictionary(x => x.x, x => x.Item2.ErrorCode);
    }
}
