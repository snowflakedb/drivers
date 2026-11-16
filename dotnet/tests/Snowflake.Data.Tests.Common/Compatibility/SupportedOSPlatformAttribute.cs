#if NETFRAMEWORK
namespace System.Runtime.Versioning;

[AttributeUsage(
    AttributeTargets.Assembly | AttributeTargets.Module | AttributeTargets.Class | AttributeTargets.Struct
    | AttributeTargets.Enum | AttributeTargets.Constructor | AttributeTargets.Method | AttributeTargets.Property
    | AttributeTargets.Field | AttributeTargets.Event | AttributeTargets.Interface | AttributeTargets.Delegate,
    AllowMultiple = true,
    Inherited = false)]
public sealed class SupportedOSPlatformAttribute : Attribute
{
    public SupportedOSPlatformAttribute(string platformName)
    {
        PlatformName = platformName;
    }

    public string PlatformName { get; }
}
#endif
