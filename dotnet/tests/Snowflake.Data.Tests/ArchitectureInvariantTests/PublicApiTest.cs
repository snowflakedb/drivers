using System.Data.Common;
using System.Diagnostics;
using System.Reflection;
using Microsoft.Extensions.Logging;
using Snowflake.Data.Core.Session;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Metadata;
using Snowflake.Data.Tests.ArchitectureInvariantTests.Utils;
using Snowflake.Data.Tests.Assertions;

namespace Snowflake.Data.Tests.ArchitectureInvariantTests;

[Trait("Category", "Architecture")]
[Trait("Category", "Unit")]
public sealed class PublicApiTest
{
    [SnowflakeFact]
    public void TestPublicTypesAllowList_NoPublicTypesOutsideAllowList()
    {
        var allowedPublicTypes = new HashSet<string>
        {
            typeof(SnowflakeDbConnection).FullName!,
            typeof(SnowflakeDbCommand).FullName!,
            typeof(SnowflakeDbCommandBuilder).FullName!,
            typeof(SnowflakeDbDataAdapter).FullName!,
            typeof(SnowflakeDbDataReader).FullName!,
            typeof(SnowflakeDbLoggerConfig).FullName!,
            typeof(SnowflakeDbParameter).FullName!,
            typeof(SnowflakeDbParameterCollection).FullName!,
            typeof(SnowflakeDbSessionPool).FullName!,
            typeof(SnowflakeDbTransaction).FullName!,
            typeof(SnowflakeActivityStarter).FullName!,
            typeof(ChangedSessionBehavior).FullName!,
            typeof(ISnowflakeCredentialManager).FullName!,
            typeof(SnowflakeCredentialManagerFactory).FullName!,
        };

        var violations = AssemblyUtil.LoadAssembly(AssembliesMetadata.RootAssembly).GetTypes()
            .Where(t => t.IsPublic || t.IsNestedPublic)
            .Select(type => type.FullName ?? type.Name)
            .Where(fullName => !allowedPublicTypes.Contains(fullName))
            .ToList();

        violations.ShouldBeEmpty("Public types not in allow-list found");
    }

    [SnowflakeTheory]
    [MemberData(nameof(PublicAreSurfaceKeys))]
    public void TestPublicApiSurface_PublicMembersMustNotGrowBeyondAllowList(Type type)
    {
        var allowedMembers = PublicApiSurface[type];
        var members = type.GetMembers(BindingFlags.Public | BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly);

        var violations = members
            .Select(FormatMemberSignature)
            .OfType<string>()
            .Where(signature => !allowedMembers.Contains(signature))
            .Select(signature => $"{type.Name}.{signature}").ToList();

        violations.ShouldBeEmpty($"New public members found on {type.Name} not in allow-list");
    }

    private static readonly HashSet<string> SnowflakeDbConnectionPublicApi =
    [
        "C:()",
        $"C:({nameof(String)})",
        $"P:get_{nameof(DbConnection.ConnectionString)}()",
        $"P:set_{nameof(DbConnection.ConnectionString)}({nameof(String)})",
        $"P:get_{nameof(DbConnection.Database)}()",
        $"P:get_{nameof(DbConnection.DataSource)}()",
        $"P:get_{nameof(DbConnection.ServerVersion)}()",
        $"P:get_{nameof(DbConnection.State)}()",
        $"M:{nameof(DbConnection.ChangeDatabase)}({nameof(String)})",
        $"M:{nameof(DbConnection.Open)}()",
        $"M:{nameof(DbConnection.OpenAsync)}({nameof(CancellationToken)})",
        $"M:{nameof(DbConnection.Close)}()",
    ];

    private static readonly HashSet<string> SnowflakeDbCommandPublicApi =
    [
        "C:()",
        $"P:get_{nameof(DbCommand.CommandText)}()",
        $"P:set_{nameof(DbCommand.CommandText)}({nameof(String)})",
        $"P:get_{nameof(DbCommand.CommandTimeout)}()",
        $"P:set_{nameof(DbCommand.CommandTimeout)}({nameof(Int32)})",
        $"P:get_{nameof(DbCommand.CommandType)}()",
        $"P:set_{nameof(DbCommand.CommandType)}({nameof(CommandType)})",
        $"P:get_{nameof(DbCommand.DesignTimeVisible)}()",
        $"P:set_{nameof(DbCommand.DesignTimeVisible)}({nameof(Boolean)})",
        $"P:get_{nameof(DbCommand.UpdatedRowSource)}()",
        $"P:set_{nameof(DbCommand.UpdatedRowSource)}({nameof(UpdateRowSource)})",
        $"M:{nameof(DbCommand.Cancel)}()",
        $"M:{nameof(DbCommand.ExecuteNonQuery)}()",
        $"M:{nameof(DbCommand.ExecuteScalar)}()",
        $"M:{nameof(DbCommand.Prepare)}()",
    ];

    private static readonly HashSet<string> SnowflakeDbDataReaderPublicApi =
    [
        $"P:get_{nameof(DbDataReader.FieldCount)}()",
        $"P:get_{nameof(DbDataReader.RecordsAffected)}()",
        $"P:get_{nameof(DbDataReader.HasRows)}()",
        $"P:get_{nameof(DbDataReader.IsClosed)}()",
        $"P:get_{nameof(DbDataReader.Depth)}()",
        $"P:get_Item({nameof(Int32)})",
        $"P:get_Item({nameof(String)})",
        $"M:{nameof(DbDataReader.Read)}()",
        $"M:{nameof(DbDataReader.IsDBNull)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetValue)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetValues)}({nameof(Object)}[])",
        $"M:{nameof(DbDataReader.GetInt64)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetInt32)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetInt16)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetDecimal)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetBoolean)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetByte)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetBytes)}({nameof(Int32)}, {nameof(Int64)}, {nameof(Byte)}[], {nameof(Int32)}, {nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetChar)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetChars)}({nameof(Int32)}, {nameof(Int64)}, {nameof(Char)}[], {nameof(Int32)}, {nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetDataTypeName)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetDateTime)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetDouble)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetFieldType)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetFloat)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetGuid)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetName)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.GetOrdinal)}({nameof(String)})",
        $"M:{nameof(DbDataReader.GetString)}({nameof(Int32)})",
        $"M:{nameof(DbDataReader.NextResult)}()",
        $"M:{nameof(DbDataReader.GetEnumerator)}()",
        $"M:{nameof(DbDataReader.Close)}()",
    ];

    private static readonly HashSet<string> SnowflakeDbParameterPublicApi =
    [
        "C:()",
        $"P:get_{nameof(DbParameter.DbType)}()",
        $"P:set_{nameof(DbParameter.DbType)}({nameof(DbType)})",
        $"P:get_{nameof(DbParameter.Direction)}()",
        $"P:set_{nameof(DbParameter.Direction)}({nameof(ParameterDirection)})",
        $"P:get_{nameof(DbParameter.IsNullable)}()",
        $"P:set_{nameof(DbParameter.IsNullable)}({nameof(Boolean)})",
        $"P:get_{nameof(DbParameter.ParameterName)}()",
        $"P:set_{nameof(DbParameter.ParameterName)}({nameof(String)})",
        $"P:get_{nameof(DbParameter.Size)}()",
        $"P:set_{nameof(DbParameter.Size)}({nameof(Int32)})",
        $"P:get_{nameof(DbParameter.SourceColumn)}()",
        $"P:set_{nameof(DbParameter.SourceColumn)}({nameof(String)})",
        $"P:get_{nameof(DbParameter.SourceColumnNullMapping)}()",
        $"P:set_{nameof(DbParameter.SourceColumnNullMapping)}({nameof(Boolean)})",
        $"P:get_{nameof(DbParameter.Value)}()",
        $"P:set_{nameof(DbParameter.Value)}({nameof(Object)})",
        $"M:{nameof(DbParameter.ResetDbType)}()",
    ];

    private static readonly HashSet<string> SnowflakeDbParameterCollectionPublicApi =
    [
        "C:()",
        $"P:get_{nameof(DbParameterCollection.Count)}()",
        $"P:get_{nameof(DbParameterCollection.SyncRoot)}()",
        $"M:{nameof(DbParameterCollection.Add)}({nameof(Object)})",
        $"M:{nameof(DbParameterCollection.AddRange)}({nameof(Array)})",
        $"M:{nameof(DbParameterCollection.Clear)}()",
        $"M:{nameof(DbParameterCollection.Contains)}({nameof(Object)})",
        $"M:{nameof(DbParameterCollection.Contains)}({nameof(String)})",
        $"M:{nameof(DbParameterCollection.CopyTo)}({nameof(Array)}, {nameof(Int32)})",
        $"M:{nameof(DbParameterCollection.GetEnumerator)}()",
        $"M:{nameof(DbParameterCollection.IndexOf)}({nameof(Object)})",
        $"M:{nameof(DbParameterCollection.IndexOf)}({nameof(String)})",
        $"M:{nameof(DbParameterCollection.Insert)}({nameof(Int32)}, {nameof(Object)})",
        $"M:{nameof(DbParameterCollection.Remove)}({nameof(Object)})",
        $"M:{nameof(DbParameterCollection.RemoveAt)}({nameof(Int32)})",
        $"M:{nameof(DbParameterCollection.RemoveAt)}({nameof(String)})",
    ];

    private static readonly HashSet<string> SnowflakeDbTransactionPublicApi =
    [
        $"P:get_{nameof(DbTransaction.IsolationLevel)}()",
        $"M:{nameof(DbTransaction.Commit)}()",
        $"M:{nameof(DbTransaction.Rollback)}()",
    ];

    private static readonly HashSet<string> SnowflakeActivityStarterPublicApi =
    [
        $"M:{nameof(SnowflakeActivityStarter.StartActivity)}({nameof(SnowflakeDbCommand)}, {nameof(String)})",
        $"M:{nameof(SnowflakeActivityStarter.SetSuccess)}({nameof(Activity)})",
        $"M:{nameof(SnowflakeActivityStarter.SetException)}({nameof(Activity)}, {nameof(Exception)})",
        $"M:{nameof(SnowflakeActivityStarter.AddTelemetryEvent)}({nameof(Activity)}, {nameof(String)})",
        $"M:{nameof(SnowflakeActivityStarter.AddTelemetryEvent)}({nameof(Activity)}, {nameof(String)}, {nameof(ActivityTagsCollection)})",
    ];

    private static readonly HashSet<string> ChangedSessionBehaviorPublicApi = [];

    private static readonly HashSet<string> ISnowflakeCredentialManagerPublicApi =
    [
        $"M:{nameof(ISnowflakeCredentialManager.GetCredentials)}({nameof(String)})",
        $"M:{nameof(ISnowflakeCredentialManager.RemoveCredentials)}({nameof(String)})",
        $"M:{nameof(ISnowflakeCredentialManager.SaveCredentials)}({nameof(String)}, {nameof(String)})",
    ];

    private static readonly HashSet<string> SnowflakeCredentialManagerFactoryPublicApi =
    [
        "C:()",
        $"M:{nameof(SnowflakeCredentialManagerFactory.UseDefaultCredentialManager)}()",
        $"M:{nameof(SnowflakeCredentialManagerFactory.UseInMemoryCredentialManager)}()",
        $"M:{nameof(SnowflakeCredentialManagerFactory.UseFileCredentialManager)}()",
        $"M:{nameof(SnowflakeCredentialManagerFactory.UseWindowsCredentialManager)}()",
        $"M:{nameof(SnowflakeCredentialManagerFactory.SetCredentialManager)}({nameof(ISnowflakeCredentialManager)})",
        $"M:{nameof(SnowflakeCredentialManagerFactory.GetCredentialManager)}()",
    ];

    private static readonly HashSet<string> SnowflakeDbCommandBuilderPublicApi =
    [
        "C:()",
        $"C:({nameof(SnowflakeDbDataAdapter)})",
        $"P:get_{nameof(DbCommandBuilder.QuotePrefix)}()",
        $"P:set_{nameof(DbCommandBuilder.QuotePrefix)}({nameof(String)})",
        $"P:get_{nameof(DbCommandBuilder.QuoteSuffix)}()",
        $"P:set_{nameof(DbCommandBuilder.QuoteSuffix)}({nameof(String)})",
    ];

    private static readonly HashSet<string> SnowflakeDbDataAdapterPublicApi =
    [
        "C:()",
        $"C:({nameof(SnowflakeDbCommand)})",
        $"C:({nameof(String)}, {nameof(SnowflakeDbConnection)})",
        $"P:get_SelectCommand()",
        $"P:set_SelectCommand({nameof(SnowflakeDbCommand)})",
    ];

    private static readonly HashSet<string> SnowflakeDbLoggerConfigPublicApi =
    [
        "C:()",
        $"M:{nameof(SnowflakeDbLoggerConfig.ResetCustomLogger)}()",
        $"M:{nameof(SnowflakeDbLoggerConfig.SetCustomLogger)}({nameof(ILogger)})",
    ];

    private static readonly HashSet<string> SnowflakeDbSessionPoolPublicApi =
    [
        "C:()",
        $"M:{nameof(SnowflakeDbSessionPool.GetPooling)}()",
        $"M:{nameof(SnowflakeDbSessionPool.GetMinPoolSize)}()",
        $"M:{nameof(SnowflakeDbSessionPool.GetMaxPoolSize)}()",
        $"M:{nameof(SnowflakeDbSessionPool.GetCurrentPoolSize)}()",
        $"M:{nameof(SnowflakeDbSessionPool.GetExpirationTimeout)}()",
        $"M:{nameof(SnowflakeDbSessionPool.GetConnectionTimeout)}()",
        $"M:{nameof(SnowflakeDbSessionPool.GetWaitForIdleSessionTimeout)}()",
        $"M:{nameof(SnowflakeDbSessionPool.ClearPool)}()",
        $"M:{nameof(SnowflakeDbSessionPool.GetChangedSession)}()",
    ];

    public static IEnumerable<object[]> PublicAreSurfaceKeys => PublicApiSurface.Keys.Select(x => new object[] { x });

    private static readonly Dictionary<Type, HashSet<string>> PublicApiSurface = new()
    {
        [typeof(SnowflakeDbConnection)] = SnowflakeDbConnectionPublicApi,
        [typeof(SnowflakeDbCommand)] = SnowflakeDbCommandPublicApi,
        [typeof(SnowflakeDbCommandBuilder)] = SnowflakeDbCommandBuilderPublicApi,
        [typeof(SnowflakeDbDataAdapter)] = SnowflakeDbDataAdapterPublicApi,
        [typeof(SnowflakeDbDataReader)] = SnowflakeDbDataReaderPublicApi,
        [typeof(SnowflakeDbLoggerConfig)] = SnowflakeDbLoggerConfigPublicApi,
        [typeof(SnowflakeDbParameter)] = SnowflakeDbParameterPublicApi,
        [typeof(SnowflakeDbParameterCollection)] = SnowflakeDbParameterCollectionPublicApi,
        [typeof(SnowflakeDbSessionPool)] = SnowflakeDbSessionPoolPublicApi,
        [typeof(SnowflakeDbTransaction)] = SnowflakeDbTransactionPublicApi,
        [typeof(SnowflakeActivityStarter)] = SnowflakeActivityStarterPublicApi,
        [typeof(ChangedSessionBehavior)] = ChangedSessionBehaviorPublicApi,
        [typeof(ISnowflakeCredentialManager)] = ISnowflakeCredentialManagerPublicApi,
        [typeof(SnowflakeCredentialManagerFactory)] = SnowflakeCredentialManagerFactoryPublicApi,
    };

    private static string? FormatMemberSignature(MemberInfo member)
    {
        return member switch
        {
            ConstructorInfo ctor => $"C:({FormatParams(ctor.GetParameters())})",
            MethodInfo { IsSpecialName: true } method when method.Name.StartsWith("get_") =>
                $"P:{method.Name}({FormatParams(method.GetParameters())})",
            MethodInfo { IsSpecialName: true } method when method.Name.StartsWith("set_") =>
                $"P:{method.Name}({FormatParams(method.GetParameters())})",
            MethodInfo { IsSpecialName: false } method =>
                $"M:{method.Name}({FormatParams(method.GetParameters())})",
            _ => null,
        };
    }

    private static string FormatParams(ParameterInfo[] parameters) =>
        string.Join(", ", parameters.Select(p => FormatTypeName(p.ParameterType)));

    private static string FormatTypeName(Type type)
    {
        if (type.IsArray)
            return $"{FormatTypeName(type.GetElementType()!)}[]";

        if (!type.IsGenericType)
            return type.Name;

        var name = type.Name.Split('`')[0];
        var args = string.Join(", ", type.GetGenericArguments().Select(FormatTypeName));
        return $"{name}<{args}>";
    }

    [SnowflakeFact]
    public void TestNoProtoTypesInPublicApi_ProtoTypesMustNotLeakThroughPublicSurface()
    {
        var violations = new List<string>();

        var publicTypes = AssemblyUtil
            .LoadAssembly(AssembliesMetadata.RootAssembly)
            .GetTypes()
            .Where(t => t.IsPublic || t.IsNestedPublic);

        foreach (var type in publicTypes)
        {
            var members = type.GetMembers(BindingFlags.Public | BindingFlags.Instance | BindingFlags.Static | BindingFlags.DeclaredOnly);

            foreach (var member in members)
            {
                var referencedTypes = GetReferencedTypes(member);
                violations.AddRange(referencedTypes
                    .Where(refType => refType.Namespace?.StartsWith(AssembliesMetadata.ProtoAssembly.Name, StringComparison.Ordinal) == true)
                    .Select(refType => $"{type.Name}.{member.Name} references {refType.FullName}"));
            }
        }

        violations.ShouldBeEmpty("Proto types must not appear in public API signatures");
    }

    private static IEnumerable<Type> GetReferencedTypes(MemberInfo member)
    {
        switch (member)
        {
            case MethodInfo method:
                yield return method.ReturnType;
                foreach (var param in method.GetParameters())
                    yield return param.ParameterType;
                break;

            case PropertyInfo property:
                yield return property.PropertyType;
                break;

            case FieldInfo field:
                yield return field.FieldType;
                break;

            case ConstructorInfo ctor:
                foreach (var param in ctor.GetParameters())
                    yield return param.ParameterType;
                break;
        }
    }
}
