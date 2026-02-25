## Add Scenario Outline / Scenario Template support + fix coverage report

### What's new

- **Scenario Outline & Scenario Template support** — The Rust validator and the Python coverage report generator now recognize `Scenario Outline:` and `Scenario Template:` keywords alongside `Scenario:`. `Examples:` tables are properly skipped during step parsing.
- **Angle bracket stripping in name matching** — Scenario Outline placeholder names like `should process <type> values` now correctly match test methods like `should_process_type_values`.

### Bugs fixed

- **Coverage report: duplicate feature name collision** — Features with the same filename in different directories (e.g., `shared/types/string.feature` vs `odbc/types/string.feature`) silently overwrote each other in the report. Now uses path-based feature IDs matching the Rust validator's approach from #229.
- **Coverage report: dead code & inaccurate docstrings** — Removed unused `feature_id_for_bd` variable; fixed `strings_match_normalized()` docstring to document all stripped characters.

### Tests added

3 regression tests covering Scenario Outline parsing, Scenario Template keyword, and Examples table row skipping.

### Verified

Manually tested by adding a fake Scenario Outline to `large_result_set.feature` and regenerating the HTML report — outline appeared correctly in all tabs, and both `string.feature` files now produce separate entries.
