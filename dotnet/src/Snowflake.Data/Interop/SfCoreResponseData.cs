namespace Snowflake.Data.Interop;

internal readonly record struct SfCoreResponseData(int Code, ArraySegment<byte> Response, byte[] Buffer);
