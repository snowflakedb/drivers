namespace Snowflake.Data.Tests.Compatibility;

/// <summary>Creates instances of a type without running any of its constructors.</summary>
/// <remarks>.NET Framework has no <c>RuntimeHelpers.GetUninitializedObject</c>, and the serializer API that replaces it there is obsolete on .NET.</remarks>
public static class UninitializedObject
{
    /// <summary>An instance of <paramref name="type"/> whose fields all have their default values.</summary>
    public static object Create(Type type) =>
#if NETFRAMEWORK
        System.Runtime.Serialization.FormatterServices.GetUninitializedObject(type);
#else
        System.Runtime.CompilerServices.RuntimeHelpers.GetUninitializedObject(type);
#endif
}
