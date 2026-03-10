# C locale support in ODBC

Jira: SNOW-3220995

We are making a bigger feature today + some refactor so focus, ask questions and make detailed plan. We are tackling ODBC locale support which impacts our c api interface and conversion.

# Problem
We need to convert all the C strings (char *) received from C API to rust string representation. The same goes for the other direction we need convert rust string to C string when we return them via C API. This is tricky because we should have different encoding for each platform:
- Darwin - UTF-8
- Windows - encoding based on detected ACP
- Linux - UTF-8 most of the time unless ASCII locale is detected

# Current state
We are doing UTF-8 or ASCII encoding for the most part but we lack support for other encodings and we perform no platform detection. Also we use hodge podge of functions to perform the encoding, we need to standardize the interface for encoding and decoding, especially at C API level.

This impacts how "W" version of ODBC api is implemented. Since we do most of the conversion in the api module we need duplication for function implementation at this level.

# What we want to do:
- Provide clear module that handles ODBC encoding / decoding in new odbc driver
- Use that module in @odbc/src/c_api.rs and make sure that api module only accepts and returns rust string (except data conversion - this should be handle at conversion module)
- Use that module in conversion implementation so we handle each write to SQL_C_CHAR correctly
- Implement W version of every function make sure we accept UTF-16 encoded strings
- Remove all the extra string conversion functions, all the conversion should be performed only by the new module
- Unskip all the tests that don't work due to incorrect encoding / decoding handling

# Planning steps:
1. Research how ODBC driver should handle encoding / decoding. Validate the assumptions
2. Understand current state of odbc driver
3. Research libraries that should be used for encoding and decoding. Follow rust best practices
4. Come up with plan for executing those changes. It would be nice if we'd deliver it in multiple PRs - optimize for review comfort and speed
