# Logging providers exercised by the .NET tests

These packages are referenced from `Snowflake.Data.Tests.Common` only. The driver
itself depends on `Microsoft.Extensions.Logging.Abstractions`.

| Provider | Packages | Version | CI frameworks | Notes |
| --- | --- | --- | --- | --- |
| log4net | `log4net`, `Microsoft.Extensions.Logging.Log4Net.AspNetCore` | 3.5.0 / 10.0.0 | Interop: net10.0, net9.0, net472. Tests: net10.0, net9.0, net8.0, net481 | Distinct TRACE/DEBUG through the log4net provider. |
| Serilog | `Serilog`, `Serilog.Extensions.Logging` | 4.4.0 / 10.0.0 | Same as log4net | In-test `ILogEventSink`. `Verbose` maps to Trace. |
| NLog | `NLog`, `NLog.Extensions.Logging` | 6.2.2 / 6.2.2 | Same as log4net | Isolated `LogFactory` plus `MemoryTarget`. |
| Microsoft Console | `Microsoft.Extensions.Logging.Console` | 10.0.14 | Same as log4net | Custom `ConsoleFormatter` captures events without swapping `Console.Out`. |
| Microsoft Debug | `Microsoft.Extensions.Logging.Debug` | 10.0.14 | Same as log4net | `DebugLogger.IsEnabled` is false unless a debugger is attached. The tests skip under a debugger; otherwise they assert the logger is disabled and that emitting a log does not throw. |
| Microsoft EventSource | `Microsoft.Extensions.Logging.EventSource` | 10.0.14 | Same as log4net | In-process `EventListener` on `Microsoft-Extensions-Logging`. Payload `Level` is the original `LogLevel`, so TRACE stays distinct from DEBUG. |
| Microsoft EventLog | `Microsoft.Extensions.Logging.EventLog` | 10.0.14 | Windows Interop net472 and Tests net481 | Windows-only. Writes under `.NET Runtime` in the Application log. Covers delivery, replacement/reset, and credential absence. No TRACE/DEBUG distinction. |
| ZLogger | `ZLogger` | 2.6.1 | Interop: net10.0, net9.0. Tests: net10.0, net9.0, net8.0 | Modern .NET only. In-memory `IAsyncLogProcessor`. |
