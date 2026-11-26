using System.Collections;
using System.Data.Common;
using Snowflake.Data.Core;

namespace Snowflake.Data.Client;

public sealed class SnowflakeDbParameterCollection : DbParameterCollection, IEnumerable<SnowflakeDbParameter>
{
    private readonly List<SnowflakeDbParameter> _parameters = [];

    public override int Count => _parameters.Count;

    public override object SyncRoot => ((ICollection)_parameters).SyncRoot;

    public override int Add(object value)
    {
        _parameters.Add(ToSnowflakeParameter(value));
        return _parameters.Count - 1;
    }

    public SnowflakeDbParameter Add(string parameterName, SFDataType dataType)
    {
        var parameter = new SnowflakeDbParameter(parameterName, dataType);
        _parameters.Add(parameter);
        return parameter;
    }

    public override void AddRange(Array values)
    {
        foreach (var value in values)
            _parameters.Add(ToSnowflakeParameter(value));
    }

    public override void Clear() => _parameters.Clear();

    public override bool Contains(object value) => value is SnowflakeDbParameter parameter && _parameters.Contains(parameter);

    public override bool Contains(string value) =>
        _parameters.Exists(p => p.ParameterName == value);

    public override void CopyTo(Array array, int index) =>
        ((ICollection)_parameters).CopyTo(array, index);

    public override IEnumerator GetEnumerator() => _parameters.GetEnumerator();

    IEnumerator<SnowflakeDbParameter> IEnumerable<SnowflakeDbParameter>.GetEnumerator() => _parameters.GetEnumerator();

    public override int IndexOf(object value) => value is SnowflakeDbParameter parameter ? _parameters.IndexOf(parameter) : -1;

    public override int IndexOf(string parameterName) =>
        _parameters.FindIndex(p => p.ParameterName == parameterName);

    public override void Insert(int index, object value) =>
        _parameters.Insert(index, ToSnowflakeParameter(value));

    public override void Remove(object value) => _parameters.Remove(ToSnowflakeParameter(value));

    public override void RemoveAt(int index) => _parameters.RemoveAt(index);

    public override void RemoveAt(string parameterName)
    {
        var idx = IndexOf(parameterName);
        if (idx >= 0)
            _parameters.RemoveAt(idx);
    }

    protected override DbParameter GetParameter(int index) => _parameters[index];

    protected override DbParameter GetParameter(string parameterName) =>
        _parameters[IndexOf(parameterName)];

    protected override void SetParameter(int index, DbParameter value) =>
        _parameters[index] = ToSnowflakeParameter(value);

    protected override void SetParameter(string parameterName, DbParameter value) =>
        _parameters[IndexOf(parameterName)] = ToSnowflakeParameter(value);

    private static SnowflakeDbParameter ToSnowflakeParameter(object value) =>
        value as SnowflakeDbParameter ?? throw new SnowflakeDbException(SFError.UNSUPPORTED_FEATURE);
}
