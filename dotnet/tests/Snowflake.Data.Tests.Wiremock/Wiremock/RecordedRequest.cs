namespace Snowflake.Data.Tests.Wiremock;

public sealed class RecordedRequest
{
    public RecordedRequest(string method, string url, string body)
    {
        Method = method;
        Url = url;
        Body = body;
    }

    public string Method { get; }

    public string Url { get; }

    public string Body { get; }
}
