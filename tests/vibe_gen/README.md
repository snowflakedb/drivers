# ODBC Test Generation Tools

Automated scripts to generate Gherkin and CATCH test scenarios for ODBC functions using Claude Code.

## Files

- `generate_gherkin_tests.sh` - Script to generate Gherkin test scenarios
- `generate_catch_tests.sh` - Script to generate CATCH C++ tests
- `generate_tests_and_submit_pr.sh` - Combined script that generates both tests and submits a PR
- `gherkin_test_prompt_template.md` - Prompt template for Gherkin tests
- `catch_test_prompt_template.md` - Prompt template for CATCH tests
- `odbc_functions.json` - Source data containing all ODBC function definitions

## Prerequisites

1. **jq** - JSON processor
   ```bash
   # Ubuntu/Debian
   sudo apt-get install jq

   # macOS
   brew install jq
   ```

2. **Claude Code CLI** - Must be installed and authenticated
   ```bash
   claude --version
   ```

## Usage

All scripts can be run from the project root directory.

### Generate Gherkin tests for a function:
```bash
./tests/vibe_gen/generate_gherkin_tests.sh SQLDriverConnect
```

### Generate CATCH tests for a function:
```bash
./tests/vibe_gen/generate_catch_tests.sh SQLDriverConnect
```

### Generate both tests and submit a PR:
```bash
./tests/vibe_gen/generate_tests_and_submit_pr.sh SQLDriverConnect
```

### Dry run (generate tests without committing):
```bash
./tests/vibe_gen/generate_tests_and_submit_pr.sh SQLDriverConnect --dry-run
```

## How It Works

1. **Reads** `odbc_functions.json` containing 76+ ODBC function definitions
2. **For each function**:
   - Extracts function name, return type, and parameters
   - Loads the prompt template from `gherkin_test_prompt_template.md`
   - Enriches template with function-specific data
   - Creates a dedicated folder for the function
   - Invokes Claude Code with the enriched prompt
   - Saves output to `<FunctionName>/<FunctionName>.feature` file
3. **Logs** all operations to `gherkin_generation.log`

## Output

- Feature files: `tests/definitions/odbc/vibe/<FunctionName>/<FunctionName>.feature`
- Log file: `gherkin_generation.log`

## Example Output Structure

```
tests/definitions/odbc/vibe/
├── SQLAllocHandle/
│   └── SQLAllocHandle.feature
├── SQLConnect/
│   └── SQLConnect.feature
├── SQLExecute/
│   └── SQLExecute.feature
├── SQLFetch/
│   └── SQLFetch.feature
└── ...
```

## Customization

### Modify the Prompt Template

Edit `gherkin_test_prompt_template.md` or `catch_test_prompt_template.md` to customize:
- Test scenario requirements
- Output format
- Coverage criteria
- Documentation style

Available placeholders:
- `{{FUNCTION_NAME}}` - Will be replaced with function name (e.g., "SQLConnect")
- `{{RETURN_TYPE}}` - Will be replaced with return type (e.g., "SQLRETURN")
- `{{PARAMETERS}}` - Will be replaced with formatted parameter list

### Adjust Processing

Edit `generate_gherkin_tests.sh` or `generate_catch_tests.sh` to:
- Change the delay between requests (currently 2 seconds)
- Modify error handling
- Add filtering for specific functions
- Customize output formatting

## Notes

- The script includes a 2-second delay between API calls to avoid rate limiting
- Failed generations are logged but won't stop the entire process
- Each function takes approximately 5-10 seconds to process
- Total processing time for all 76 functions: ~10-15 minutes

## Troubleshooting

### "jq is not installed"
Install jq using the commands in Prerequisites section

### "claude command not found"
Ensure Claude Code CLI is installed and in your PATH

### Rate limiting errors
Increase the `sleep` value in the script (line ~115)

### Out of memory
Process functions in batches by filtering the JSON input
