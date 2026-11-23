extern alias OldDriver;
using System.Reflection;
using Xunit.Sdk;
using OldSnowflakeDbConnection = OldDriver::Snowflake.Data.Client.SnowflakeDbConnection;

namespace Snowflake.Data.Tests.E2E;

#if NETFRAMEWORK
internal sealed class FaultInjectingChunkParserFactory : IDisposable
{
    public int ParsersRequested => 0;

    public static FaultInjectingChunkParserFactory Install(int failuresToInject) =>
        throw SkipException.ForSkip("System.Reflection.DispatchProxy is not available on .NET Framework");

    public void Dispose()
    {
    }
}
#else
internal sealed class FaultInjectingChunkParserFactory : IDisposable
{
    private readonly FieldInfo _instanceField;
    private readonly object? _previous;
    private readonly ParserFactoryProxy _proxy;

    private FaultInjectingChunkParserFactory(FieldInfo instanceField, ParserFactoryProxy proxy)
    {
        _instanceField = instanceField;
        _previous = instanceField.GetValue(null);
        _proxy = proxy;
        _instanceField.SetValue(null, proxy);
    }

    public int ParsersRequested => _proxy.ParsersRequested;

    public static FaultInjectingChunkParserFactory Install(int failuresToInject)
    {
        var oldAssembly = typeof(OldSnowflakeDbConnection).Assembly;
        var factoryType = GetOldType(oldAssembly, "Snowflake.Data.Core.ChunkParserFactory");
        var factoryInterface = GetOldType(oldAssembly, "Snowflake.Data.Core.IChunkParserFactory");
        var parserInterface = GetOldType(oldAssembly, "Snowflake.Data.Core.IChunkParser");
        var reusableParserType = GetOldType(oldAssembly, "Snowflake.Data.Core.ReusableChunkParser");
        var instanceField = factoryType.GetField("Instance", BindingFlags.Public | BindingFlags.Static)
                            ?? throw new MissingFieldException(factoryType.FullName, "Instance");

        var proxy = (ParserFactoryProxy)CreateProxy(factoryInterface, typeof(ParserFactoryProxy));
        proxy.Configure(failuresToInject, reusableParserType, () => CreateProxy(parserInterface, typeof(ThrowingParserProxy)));
        return new FaultInjectingChunkParserFactory(instanceField, proxy);
    }

    public void Dispose() => _instanceField.SetValue(null, _previous);

    private static Type GetOldType(Assembly assembly, string name) =>
        assembly.GetType(name, throwOnError: true)!;

    private static object CreateProxy(Type interfaceType, Type proxyType)
    {
        var create = typeof(DispatchProxy)
            .GetMethods(BindingFlags.Public | BindingFlags.Static)
            .Single(m => m.Name == nameof(DispatchProxy.Create)
                         && m.IsGenericMethodDefinition
                         && m.GetGenericArguments().Length == 2);
        return create.MakeGenericMethod(interfaceType, proxyType).Invoke(null, null)!;
    }
}

internal class ParserFactoryProxy : DispatchProxy
{
    private int _parsersRequested;
    private int _failuresToInject;
    private Type? _reusableParserType;
    private Func<object>? _createThrowingParser;

    public int ParsersRequested => Volatile.Read(ref _parsersRequested);

    public void Configure(int failuresToInject, Type reusableParserType, Func<object> createThrowingParser)
    {
        _failuresToInject = failuresToInject;
        _reusableParserType = reusableParserType;
        _createThrowingParser = createThrowingParser;
    }

    protected override object? Invoke(MethodInfo? targetMethod, object?[]? args)
    {
        if (targetMethod?.Name != "GetParser")
            throw new NotSupportedException(targetMethod?.Name);

        if (Interlocked.Increment(ref _parsersRequested) <= _failuresToInject)
            return _createThrowingParser!();

        return Activator.CreateInstance(
            _reusableParserType!,
            BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic,
            binder: null,
            args: new[] { args![1] },
            culture: null);
    }
}

internal class ThrowingParserProxy : DispatchProxy
{
    protected override object? Invoke(MethodInfo? targetMethod, object?[]? args) =>
        throw new InvalidOperationException("json parsing error.");
}
#endif
