using Microsoft.Extensions.Logging;
using Snowflake.Data.Logging;

namespace Snowflake.Data.Tests.Logging;

[CollectionDefinition("CustomLogger", DisableParallelization = true)]
public sealed class CustomLoggerCollection : ICollectionFixture<CustomLoggerCollection.Fixture>
{
    public sealed class Fixture : IDisposable
    {
        private readonly ILogger _previous;

        public Fixture()
        {
            _previous = CustomLoggerHolder.Current;
        }

        public void Dispose() => CustomLoggerHolder.Set(_previous);
    }
}
