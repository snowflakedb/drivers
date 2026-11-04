namespace Snowflake.Data.Attributes;

internal class SFErrorAttribute : Attribute
{
    public int ErrorCode { get; set; }
}
