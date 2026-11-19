using System.Collections.Concurrent;
using System.Reflection;
using Snowflake.Data.Tests.Compatibility;
using Xunit.Sdk;
using NewISnowflakeCredentialManager = Snowflake.Data.ISnowflakeCredentialManager;
using NewSnowflakeCredentialManagerFactory = Snowflake.Data.SnowflakeCredentialManagerFactory;
using OldISnowflakeCredentialManager = Snowflake.Data.Client.ISnowflakeCredentialManager;
using OldSnowflakeCredentialManagerFactory = Snowflake.Data.Client.SnowflakeCredentialManagerFactory;

namespace Snowflake.Data.Tests.Reference;

internal sealed class CredentialStore
{
    private readonly string? _initialToken;
    private readonly ConcurrentDictionary<string, string> _tokens = new();
    private readonly ConcurrentDictionary<string, bool> _modifiedKeys = new();
    private readonly ConcurrentQueue<string> _keysReturningInitialToken = new();
    private readonly ConcurrentQueue<string> _removedKeys = new();

    public CredentialStore(string? initialToken) => _initialToken = initialToken;

    public IReadOnlyCollection<string> KeysReturningInitialToken => _keysReturningInitialToken.ToArray();

    public IReadOnlyCollection<string> RemovedKeys => _removedKeys.ToArray();

    public string Get(string key)
    {
        if (_tokens.TryGetValue(key, out var token))
            return token;
        if (_initialToken is null || _modifiedKeys.ContainsKey(key))
            return string.Empty;
        _keysReturningInitialToken.Enqueue(key);
        return _initialToken;
    }

    public void Remove(string key)
    {
        _modifiedKeys[key] = true;
        _tokens.TryRemove(key, out _);
        _removedKeys.Enqueue(key);
    }

    public void Save(string key, string token)
    {
        _modifiedKeys[key] = true;
        _tokens[key] = token;
    }
}

#if NETFRAMEWORK
internal sealed class CredentialManagerOverride : IDisposable
{
    public static CredentialManagerOverride Install(CredentialStore store) =>
        throw SkipException.ForSkip("System.Reflection.DispatchProxy is not available on .NET Framework");

    public void Dispose()
    {
    }
}
#else
internal sealed class CredentialManagerOverride : IDisposable
{
    private readonly object _original;

    private CredentialManagerOverride(object original) => _original = original;

    public static CredentialManagerOverride Install(CredentialStore store)
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
        {
            var oldProxy = DispatchProxy.Create<OldISnowflakeCredentialManager, CredentialManagerProxy>();
            ((CredentialManagerProxy)(object)oldProxy).Store = store;
            var originalOld = OldSnowflakeCredentialManagerFactory.GetCredentialManager();
            OldSnowflakeCredentialManagerFactory.SetCredentialManager(oldProxy);
            return new CredentialManagerOverride(originalOld);
        }

        var newProxy = DispatchProxy.Create<NewISnowflakeCredentialManager, CredentialManagerProxy>();
        ((CredentialManagerProxy)(object)newProxy).Store = store;
        var originalNew = NewSnowflakeCredentialManagerFactory.GetCredentialManager();
        NewSnowflakeCredentialManagerFactory.SetCredentialManager(newProxy);
        return new CredentialManagerOverride(originalNew);
    }

    public void Dispose()
    {
        if (TestDriver.Current == TestDriver.Kind.Old)
            OldSnowflakeCredentialManagerFactory.SetCredentialManager((OldISnowflakeCredentialManager)_original);
        else
            NewSnowflakeCredentialManagerFactory.SetCredentialManager((NewISnowflakeCredentialManager)_original);
    }
}

internal class CredentialManagerProxy : DispatchProxy
{
    public CredentialStore? Store { get; set; }

    protected override object? Invoke(MethodInfo? targetMethod, object?[]? args)
    {
        var store = Store ?? throw new InvalidOperationException("The credential store is not set");
        switch (targetMethod?.Name)
        {
            case "GetCredentials":
                return store.Get((string)args![0]!);
            case "RemoveCredentials":
                store.Remove((string)args![0]!);
                return null;
            case "SaveCredentials":
                store.Save((string)args![0]!, (string)args[1]!);
                return null;
            default:
                throw new NotSupportedException(targetMethod?.Name);
        }
    }
}
#endif
