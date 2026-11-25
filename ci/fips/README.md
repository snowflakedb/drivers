# FIPS candidate evidence

`release_evidence.py` writes one bundle for a native library built with the `fips` feature. `fips_artifacts` in `.github/workflows/test-rust-core.yml` does this for the ODBC library and the Node.js addon after `check_linked_crypto.py` passes.

The bundle is a GitHub Actions candidate. It hashes the unstripped library this job inspected. Release Engineering later signs and notarizes ODBC installers, and GPG-signs JDBC during publication, which changes customer bytes. A candidate attestation does not cover those published files. Python wheels are the exception today: Jenkins publishes the candidate bytes unchanged and adds a detached signature.

## Bundle

| File | Contents |
| --- | --- |
| `sbom.cdx.json` | CycloneDX 1.5 for the bridge crate, with `--features fips` and without `--all-features`. `aws-lc-fips-sys` stays at the exact `sf_core` pin. An `AWS-LC FIPS` component carries the runtime version from `EXPECTED_FIPS_MODULE_VERSION`. `aws-lc-sys` stays in the BOM when Cargo resolves it, with `snowflake:aws-lc-sys` saying why it is not linkage evidence. |
| `linkage.txt` | `check_linked_crypto.py` output for the same file. |
| `dumpbin.txt`, `linker-map.txt` | Windows only. |
| `hashes.json` | SHA-256 of the native file, the SBOM, and the FIPS module that file loads. Windows records the DLL named by `dumpbin /dependents`. macOS records the `libaws_lc_fips_*_crypto.dylib` named by `otool -L`. Each companion includes the path of that file. Linux links the module into the library, so it has no companion. |
| `toolchain.json` | `rustc -vV`, Cargo, `cargo-cyclonedx`, and `CC`/`CXX` when set. |
| `manifest.json` | `stage` is `gha-candidate` and `covers_releng_published_bytes` is false. |

On macOS and Windows that module is the `aws-lc-fips-sys` build-script output under `target/<triple>/release/build` when the build did not copy it beside the library. `hashes.json` records that path. An older cached copy of the same filename stays out.

`ring`, `openssl`, `openssl-sys`, `openssl-src`, `openssl-macros`, and `native-tls` fail the SBOM check. The packaged status API is withheld, so `self_check.status` is `not-run`.

Outside pull requests, the job also runs `actions/attest@v4` twice per library: SLSA provenance, then an SBOM attestation whose subject is that same candidate file. `actions/attest@v4` has to be on the snowflakedb org allowlist before a public workflow runs it:

https://github.com/snowflakedb/self-serve-snowflakedb/blob/26ecbd5af4827db3e40fe9cdbad9777d7abd3e35/configs/snowflakedb/default/default_actions.yml

## Package bundle

`package_evidence.py` writes a bundle for one finished package file (`.rpm`, `.deb`, `.tar.gz`, `.pkg`, or `.msi`). The subject is that package file. `sbom.cdx.json` lists the files the package installs, and `manifest.json` sets `stage` to `gha-package-candidate` with `covers_releng_published_bytes` false. The FIPS rows of the RPM, deb, tgz, macOS pkg, and Windows x64 MSI jobs run it after the package file exists and upload the bundle next to that package. Outside pull requests those same jobs also run `actions/attest@v4` on that package file and on its SBOM. The signed digest is the CI package. Release Engineering signing and notarization still change the bytes customers download.

## Other packages

The FIPS RPM, deb, tgz, macOS pkg, and Windows x64 MSI jobs extract `libsfodbc.so`, `libsfodbc.dylib`, or `sfodbc.dll` from the package file they hashed and run `check_linked_crypto.py` on that extracted file. The Windows check also reads the linker map from the cargo rustc that produced the DLL. The earlier linkage check still reads the library in the Cargo target directory, before packaging.

The JDBC jar workflow (`.github/workflows/_build-jdbc-fatjar.yml`) builds a FIPS `jdbc_bridge` for Linux glibc x86_64 and aarch64, macOS x86_64 and arm64, and Windows x64, with no musl build. Each `build_native_fips` row runs `check_linked_crypto.py` and `fips-release-evidence` on the unstripped bridge in the Cargo target directory, then stages the companion libraries listed in that bundle's `hashes.json` after checking their hashes, plus a `companions.txt` in the same order. The macOS rows then rewrite the companion install names to `@loader_path` and re-sign ad hoc, so the jar holds different bytes than the bundle hashed; `staged-files.sha256` in the same evidence artifact records the digests of the staged files that the jar ships. `assemble_fips` and `assemble_thin_fips` package only those natives, as `snowflake-jdbc-native-all-fips` and `snowflake-jdbc-native-fips`. `test_fips` runs `FipsBuildTest` and `SnowflakeDriverTlsStatusTest` with `JDBC_EXPECT_FIPS_BUILD=true` on the same five platforms, once from the assembled fat jar and once from that platform's thin classifier jar. `-PnativeJar` puts the jar ahead of the build output on the test classpath and removes `CORE_PATH`, so `NativeLibraryLoader` extracts the bridge and its companions from the jar and connects, and `JDBC_EXPECT_NATIVE_JAR` (the path of that jar) makes `SnowflakeDriverTlsStatusTest` fail if `CORE_PATH` is set or the bundled native resolves to any other jar. A failing run still copies its XML reports, and the second jar runs even if the first fails. `test_fips` fails if either suite skipped. It runs the FIPS-only scenarios (TLS query, 10,000-row fetch, PUT, key-pair auth, rejection of disabled certificate verification and 3DES keys), not the full JDBC suite. The `fips_test_secrets` job states once whether the run has secrets (a fork PR or a `snowflakedb` run has none); `test_fips` and `fips_release_gate` both read it. `fips_release_gate` needs `smoke_fips` and `test_fips` and fails unless both succeeded (a skipped `test_fips` is accepted only without secrets). When `test_fips` succeeded it uploads `jdbc-fips-release-gate`, whose `fips-jars.sha256` lists the digest of every file in `jdbc-fatjar-fips` and `jdbc-thin-fips`. The artifact exists only when `test_fips` ran and passed, so a release step that requires it releases only jars the tests ran against. That release step is outside this repository, and nothing signs the jars yet.

Python and .NET release workflows do not build FIPS packages yet. A FIPS lane calls `.github/actions/fips-release-evidence` with the bridge `Cargo.toml`, the same features and target as the build, and the unstripped library before any byte-changing sign or strip step. npm has no lockfile in this repo, so the Node bundle does not include an npm dependency SBOM.
