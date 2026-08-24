using Snowflake.Data.Proto;

namespace Snowflake.Data.Tests.Assertions;

// TODO will be used in the future
internal static class ServiceExceptionAssert
{
    public static void HasErrorKind(ServiceException exception, ErrorKind expected)
    {
        Assert.Equal(expected, exception.Error.Kind);
    }

    public static void HasVendorCode(ServiceException exception, int expectedVendorCode)
    {
        Assert.True(exception.Error.HasVendorCode, "Expected DriverException to have a VendorCode.");
        Assert.Equal(expectedVendorCode, exception.Error.VendorCode);
    }

    public static void HasErrorKindInExceptionChain(Exception exception, ErrorKind expected)
    {
        var exceptions = CollectExceptions(exception);
        var kinds = exceptions
            .OfType<ServiceException>()
            .Select(x => x.Error.Kind)
            .Distinct()
            .ToArray();
        Assert.Contains(expected, kinds);
    }

    public static void HasVendorCodeInExceptionChain(Exception exception, int expectedVendorCode)
    {
        var exceptions = CollectExceptions(exception);
        Assert.Contains(exceptions, e =>
            e is ServiceException se && se.Error.HasVendorCode && se.Error.VendorCode == expectedVendorCode);
    }

    public static void HasMessageInExceptionChain(Exception exception, string expected)
    {
        var exceptions = CollectExceptions(exception);
        Assert.Contains(exceptions, e => e.Message.Contains(expected));
    }

    private static List<Exception> CollectExceptions(Exception? exception)
    {
        var collected = new List<Exception>();
        if (exception is null)
            return collected;

        switch (exception)
        {
            case AggregateException aggregate:
                var inner = aggregate.Flatten().InnerExceptions;
                collected.AddRange(inner);
                collected.AddRange(inner
                    .Where(e => e.InnerException != null)
                    .SelectMany(e => CollectExceptions(e.InnerException)));
                break;
            default:
                collected.AddRange(CollectExceptions(exception.InnerException));
                collected.Add(exception);
                break;
        }

        return collected;
    }
}
