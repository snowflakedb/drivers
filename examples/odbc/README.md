# ODBC Examples

This directory contains ODBC examples demonstrating various operations with the Snowflake Universal Driver.

## Building

Compile all examples with:

```bash
./run.sh
```

## Examples

### select_1
Basic SELECT operation that executes "SELECT 1" and retrieves the result.

```bash
./example select_1 "connection_string"
```

### put_file
Demonstrates file upload functionality using the PUT command. This example:
- Creates a temporary CSV file with sample data
- Creates a temporary stage 
- Uploads the file to the stage using PUT
- Displays the upload results (status, file sizes, compression info)
- Lists the stage contents to verify the upload
- Cleans up temporary files

```bash
./example put_file "connection_string"
```

### get_file
Demonstrates file download functionality using the GET command. This example:
- Creates a temporary CSV file and uploads it to a stage (for demonstration)
- Downloads the file from the stage using GET
- Displays the download results (status, file size)
- Verifies the downloaded file exists
- Cleans up temporary files and directories

```bash
./example get_file "connection_string"
```

## Connection String Format

The connection string should follow the ODBC format for Snowflake, for example:
```
"DRIVER={SnowflakeDriver};SERVER=your_account.snowflakecomputing.com;UID=your_username;PWD=your_password;DATABASE=your_database;SCHEMA=your_schema;WAREHOUSE=your_warehouse"
```

## Notes

- All examples include proper error handling using the macros defined in `macros.h`
- PUT operations automatically compress files (typically to .gz format)
- GET operations download compressed files and maintain the compression
- Temporary stages are created for demonstration and don't persist beyond the session
- File paths use forward slashes even on Windows (standard for file:// URIs)
