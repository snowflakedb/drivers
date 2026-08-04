# Driver Troubleshooting Mode

Troubleshooting mode writes **all** log events (core + wrapper, every level)
to a file on disk, **independent of the wrapper's own log configuration**.
It is designed for support scenarios where a customer needs to capture
diagnostic logs without changing application code.

## Activation

Set the environment variable **before** the driver initializes:

```
SNOWFLAKE_TROUBLESHOOTING_ENABLED=true
```

The flag is read **once** at process start and is immutable for the
lifetime of the process. There is no connection-parameter override and
no runtime toggle - activating troubleshooting currently requires a
process restart.

> **Future improvement:** Allow enabling troubleshooting mode at runtime
> (e.g. via a connection parameter or bridge reconfiguration) so
> customers can turn it on without restarting the process.

## Log location

| Variable                                | Default                   | Notes                    |
|-----------------------------------------|---------------------------|--------------------------|
| `SNOWFLAKE_TROUBLESHOOTING_REPORT_PATH` | Current working directory | Must be set before init. |

The driver creates the directory if it does not exist.
Logs are written to a single file named `sf_driver_troubleshooting.log`
(no rotation, no date suffix). On Unix the file is created with owner-only
permissions (`0600`).

## What gets logged

An unfiltered file-appender layer is added to the logging subscriber stack.
The wrapper-side `CoreLogger` pre-filter gate is bypassed, so **all**
events (core and wrapper, every level) reach the file regardless of
the wrapper's configured log level.

## Note on Easy Logging

The legacy drivers expose an "Easy Logging" feature that lets customers
enable file-based logging via a config file (`clientConfigFile`
connection parameter or a well-known path like `~/.snowflake/config.json`;
the Python driver uses TOML, e.g. `config.toml`).

Troubleshooting mode replaces Easy Logging - it serves the same purpose
(capture logs to disk without code changes) with a simpler activation model
(a single environment variable instead of a config file).

> See [Node.js driver logging docs](https://docs.snowflake.com/en/developer-guide/node-js/nodejs-driver-logs).

## Interaction with diagnostics (SnowCD)

When troubleshooting mode is active and a `DiagnosticConfig` has no
explicit `log_path`, the diagnostic report is written to the same
directory as the troubleshooting log. This keeps all support artifacts
co-located.
