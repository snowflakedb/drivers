using System.Text;
using Xunit.Sdk;

namespace Snowflake.Data.Tests.Assertions;

public static class AssertExtensions
{
    public static void ShouldBeEquivalent<T>(this IEnumerable<T> actual, IEnumerable<T> expected) =>
        actual.ShouldBeEquivalent(expected, null!);

    public static void ShouldBeEquivalent<T>(this IEnumerable<T> actual, IEnumerable<T> expected, IEqualityComparer<T> comparer)
    {
        var leftMinusRight = actual.Except(expected, comparer);
        var rightMinusLeft = expected.Except(actual, comparer);

        if (!leftMinusRight.Any() && !rightMinusLeft.Any())
            return;

        var error = new StringBuilder("Expected collections to be equivalent.\n");
        foreach (var item in leftMinusRight)
            error.AppendLine($"Found, but didn't expect: {item!.ToString()}");

        foreach (var item in rightMinusLeft)
            error.AppendLine($"Expected, but didn't find: {item!.ToString()}");

        throw new XunitException(error.ToString());
    }

    public static void ShouldBeEmpty<T>(this IEnumerable<T> collection, string message)
        => collection.ShouldBeEmpty(_ => message);

    public static void ShouldBeEmpty<T>(this IEnumerable<T> collection, Func<T, string> messageFmt)
    {
        var items = collection as IReadOnlyCollection<T> ?? collection.ToArray();
        if (items.Count == 0)
            return;

        var errorMessages = items.Select(messageFmt);
        throw new XunitException($"Collection was expected to be empty! \n{string.Join("\n", errorMessages)}");
    }

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
