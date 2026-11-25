using Xunit.Sdk;

[assembly: AssemblyFixture(typeof(Snowflake.Data.Tests.E2E.Config.GlobalState.DatabaseFixture))]
[assembly: Parallelization(Mode = ParallelMode.All, MaxThreads = 48)]
