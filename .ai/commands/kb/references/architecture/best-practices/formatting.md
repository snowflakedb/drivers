---
description: Which auto-formatter belongs to each subsystem of this repo, and how to tell from a set of changed files which ones to run before committing.
no-pointer: true
---

# Format Before Commit

Run this before `git commit` to auto-fix formatting across every subsystem that
has staged changes.

## Step 1 — Identify affected subsystems

```bash
git diff --name-only --cached
```

If nothing is staged yet (e.g. running proactively), use:

```bash
git diff --name-only HEAD
```

Map the output to subsystems:

| Files matching              | Subsystem  | Formatter to run                                      |
|-----------------------------|------------|-------------------------------------------------------|
| `*.rs`                      | Rust       | `cargo fmt --all`                                     |
| `*.c`, `*.cpp`, `*.h`       | C/C++      | `clang-format -i <files>` (exclude `nanoarrow_cpp/`)  |
| `python/**`                 | Python     | `cd python && hatch run precommit:fix`                |
| `jdbc/**`                   | JDBC/Java  | `cd jdbc && bash ./gradlew spotlessApply`             |
| `nodejs/**`                 | Node.js    | `cd nodejs && npm run fmt:fix`                        |

## Step 2 — Run only the relevant formatters

Run each formatter whose subsystem has at least one staged file. Skip subsystems
with no staged files — don't run the Gradle formatter when only Rust changed.

### Rust
```bash
cargo fmt --all
```

### C/C++ (ODBC)
Collect the staged C/C++ files (skip the vendored nanoarrow path), then format:
```bash
git diff --name-only --cached | grep -E '\.(c|cpp|h)$' | grep -v 'nanoarrow_cpp' | xargs -r clang-format -i
```

### Python
```bash
cd python && hatch run precommit:fix
```

### JDBC / Java
```bash
cd jdbc && bash ./gradlew spotlessApply
```

### Node.js
```bash
cd nodejs && npm run fmt:fix
```

## Step 3 — Stage formatting changes

After all formatters run, re-stage any files they modified:

```bash
git add -u
```

Then proceed with the commit.

## Notes

- Run formatters in parallel if multiple subsystems are affected — they are
  independent and don't share state.
- If a formatter is not installed (e.g. `clang-format` missing), skip it and
  note the skip; don't block the commit.
- The full pre-commit suite (including linters, type checks, and validators) is
  heavier than just formatting. This targets format-only fixes. For a full
  pre-commit dry-run use `pre-commit run --all-files`.

## Rust macro style

In `format!` and `assert!`, inline the variable in the brace when it is a plain
binding — `cargo fmt` does not do this for you.

```rust
// Instead of
format!("Failed to fetch PUT result for {}", filename)
// Write
format!("Failed to fetch PUT result for {filename}")
```
