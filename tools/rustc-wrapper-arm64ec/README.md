# rustc-wrapper-arm64ec

A `RUSTC_WRAPPER` that fixes the `arrow-buffer` crate's compilation under the
`arm64ec-pc-windows-msvc` target by injecting `--cfg target_arch="aarch64"`
into **that one crate's** rustc invocation.

## Why this exists

`arrow-buffer` defines `pub const ALIGNMENT: usize` via an exhaustive list of
`#[cfg(target_arch = "...")]` blocks - x86, x86_64, aarch64, etc. There is no
fallback arm and no feature flag escape hatch. For
`target_arch = "arm64ec"`, no branch matches, the constant is never defined,
and `pub use alignment::ALIGNMENT;`

## How it works

When `RUSTC_WRAPPER=<path-to-this-binary>` is set, cargo runs every rustc
invocation through us. We inspect the args, and:

- If the args contain **both** `--crate-name arrow_buffer` **and**
  `--target arm64ec-pc-windows-msvc`, we append two extra rustc flags:
  - `--allow=explicit_builtin_cfgs_in_flags` (silence the lint that
    correctly warns you about overriding a builtin cfg — see safety audit)
  - `--cfg target_arch="aarch64"`
- For every other rustc invocation, we pass arguments through unchanged.

`rustc` sees two `target_arch` cfg values when compiling `arrow-buffer`:
the builtin `arm64ec` (from `--target`) and our injected `aarch64`. Only
the `#[cfg(target_arch = "aarch64")]` block matches → `ALIGNMENT = 1 << 6`
becomes defined. Nothing else is selected;

## Re-audit on every arrow-buffer bump

Whenever the `arrow-buffer` version resolved by `Cargo.lock` changes:

```bash
rg 'target_arch' "$(cargo download arrow-buffer@<version> --output -)/src"
# or, after building locally:
rg 'target_arch' ~/.cargo/registry/src/*/arrow-buffer-<version>/src
```

If anything new appears under `cfg(target_arch = "aarch64")` (especially
`asm!`, `std::arch::aarch64::*`, or NEON intrinsics), reassess before
bumping. The safe-to-merge case is "still only the ALIGNMENT constant."

## Local development

```bash
# Build the wrapper once (host toolchain, no target flag).
cargo build --release --manifest-path tools/rustc-wrapper-arm64ec/Cargo.toml

# Point cargo at it for an arm64ec build.
# Linux/macOS shell:
RUSTC_WRAPPER="$PWD/tools/rustc-wrapper-arm64ec/target/release/rustc-wrapper-arm64ec" \
  cargo build --target arm64ec-pc-windows-msvc --package odbc

# Windows PowerShell:
$env:RUSTC_WRAPPER = "$PWD\tools\rustc-wrapper-arm64ec\target\release\rustc-wrapper-arm64ec.exe"
cargo build --target arm64ec-pc-windows-msvc --package odbc
```

Set `RUSTC_WRAPPER_ARM64EC_VERBOSE=1` to log a line whenever the wrapper
injects the extra cfg (useful for confirming CI is using it).

## Why not just `[patch.crates-io]`?

We considered three alternatives and rejected each for the reasons below:

| Option                                            | Why we didn't pick it                                                                           |
|---------------------------------------------------|-------------------------------------------------------------------------------------------------|
| Vendor `arrow-buffer` in this repo                | ~3,500 LOC of foreign Apache-2.0 source pulled into the tree for a 2-line patch                 |
| `[patch.crates-io]` → `snowflakedb/arrow-rs` fork | New public repo to provision + maintain; CI now depends on a github.com clone in our trust path |

The wrapper is the surgical option: ~120 LOC of Rust in this repo, no new
external dependencies, and zero impact on any crate other than
`arrow-buffer`.
