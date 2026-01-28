from snowflake.connector._internal.sf_dirs import _resolve_platform_dirs


# Snowflake's central platform dependent directories, if the folder
# ~/.snowflake/ (customizable by the environment variable SNOWFLAKE_HOME) exists
# we use that folder for everything. Otherwise, we fall back to platformdirs
# defaults. Please see comments in sf_dir.py for more information.

DIRS = _resolve_platform_dirs()

# Snowflake's configuration files. By default, platformdirs will resolve
# them to these places depending on OS:
#   * Linux: `~/.config/snowflake/filename` but can be updated with XDG vars
#   * Windows: `%USERPROFILE%\AppData\Local\snowflake\filename`
#   * Mac: `~/Library/Application Support/snowflake/filename`
CONNECTIONS_FILE = DIRS.user_config_path / "connections.toml"
CONFIG_FILE = DIRS.user_config_path / "config.toml"
