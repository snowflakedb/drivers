namespace Snowflake.Data.Tests;

public static class TestDriver
{
    public enum Kind
    {
        New,
        Old,
    }

    public static ITestConnectionFactory? OverriddenFactory { get; set; }

    public static Kind Current =>
        OverriddenFactory?.Driver ?? Kind.New;
}
