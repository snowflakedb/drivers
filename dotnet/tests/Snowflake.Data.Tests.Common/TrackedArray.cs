using System.Collections.Concurrent;
using System.Reflection;
using System.Reflection.Emit;
using Apache.Arrow;

namespace Snowflake.Data.Tests;

/// <summary>
/// Creates an Arrow array that is the same concrete type as the array it wraps and reports when it is disposed.
/// The type is emitted at run time as a subclass of the wrapped array's type, so code that matches on the
/// concrete Arrow type (<c>Int64Array</c>, <c>StringArray</c>, ...) still recognizes it.
/// </summary>
public static class TrackedArray
{
    private static readonly ModuleBuilder Module = AssemblyBuilder
        .DefineDynamicAssembly(new AssemblyName("Snowflake.Data.Tests.TrackedArrays"), AssemblyBuilderAccess.Run)
        .DefineDynamicModule("Snowflake.Data.Tests.TrackedArrays");

    private static readonly ConcurrentDictionary<Type, ConstructorInfo> Constructors = new();

    public static TArray Create<TArray>(TArray inner, Action onDispose)
        where TArray : Apache.Arrow.Array
    {
        var constructor = Constructors.GetOrAdd(typeof(TArray), Emit);
        return (TArray)constructor.Invoke([inner.Data, new Action<bool>(disposing =>
        {
            if (disposing)
                onDispose();
        })]);
    }

    private static ConstructorInfo Emit(Type baseType)
    {
        var baseConstructor = baseType.GetConstructor(
                BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic,
                null,
                [typeof(ArrayData)],
                null)
            ?? throw new InvalidOperationException($"{baseType.Name} has no constructor taking ArrayData.");
        var baseDispose = baseType.GetMethod(
                "Dispose",
                BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic,
                null,
                [typeof(bool)],
                null)
            ?? throw new InvalidOperationException($"{baseType.Name} has no Dispose(bool).");

        var type = Module.DefineType(
            $"Tracked{baseType.Name}_{Guid.NewGuid():N}",
            TypeAttributes.Public | TypeAttributes.Sealed,
            baseType);
        var hook = type.DefineField("_onDispose", typeof(Action<bool>), FieldAttributes.Private);

        var constructor = type.DefineConstructor(
            MethodAttributes.Public,
            CallingConventions.Standard,
            [typeof(ArrayData), typeof(Action<bool>)]);
        var il = constructor.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Ldarg_1);
        il.Emit(OpCodes.Call, baseConstructor);
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Ldarg_2);
        il.Emit(OpCodes.Stfld, hook);
        il.Emit(OpCodes.Ret);

        var dispose = type.DefineMethod(
            "Dispose",
            MethodAttributes.Family | MethodAttributes.Virtual | MethodAttributes.HideBySig,
            typeof(void),
            [typeof(bool)]);
        il = dispose.GetILGenerator();
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Ldarg_1);
        il.Emit(OpCodes.Call, baseDispose);
        il.Emit(OpCodes.Ldarg_0);
        il.Emit(OpCodes.Ldfld, hook);
        il.Emit(OpCodes.Ldarg_1);
        il.Emit(OpCodes.Callvirt, typeof(Action<bool>).GetMethod(nameof(Action<bool>.Invoke))!);
        il.Emit(OpCodes.Ret);
        type.DefineMethodOverride(dispose, baseDispose);

        return type.CreateType().GetConstructor([typeof(ArrayData), typeof(Action<bool>)])!;
    }
}
