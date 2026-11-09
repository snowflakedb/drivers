namespace Snowflake.Data.Tests;

public static class TestDriver
{
    public enum Kind
    {
        New,
        Old,
    }

    public static Kind Current =>
        ITFixture.OverriddenFactory?.Driver ?? Kind.New;
}
