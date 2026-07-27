using System.Text;
using Xunit.Sdk;

namespace Snowflake.Data.Tests.Assertions;

// TODO will be used in the future
public static class AssertExtensions
{
    public static void NotEmptyString(string actual)
    {
        Assert.False(string.IsNullOrEmpty(actual));
    }

    public static void Equal<T>(T expected, T actual, string message)
    {
        if (expected!.Equals(actual))
            return;

        throw new XunitException($"Expected {expected}, actual: {actual} \n" + message);
    }

    public static void AnySucceeds(params Action[] assertions)
    {
        var failedMessages = new StringBuilder();
        foreach (var action in assertions)
        {
            try
            {
                action();
                return;
            }
            catch (XunitException ex)
            {
                failedMessages.AppendLine(ex.Message);
            }
        }

        throw new XunitException(failedMessages.ToString());
    }
}
