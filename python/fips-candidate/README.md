# Snowflake Connector for Python FIPS-TLS candidate

This **experimental, unvalidated candidate** is a separately named distribution
of `snowflake.connector`. It builds `python_bridge` with the `fips-tls` Cargo
feature, but **does not claim FIPS 140-3 compliance or certification**.
`aws-lc-fips-sys` 0.13.17 embeds AWS-LC-FIPS 3.6.0, whereas NIST certificate
#5314 applies to 3.1.0. Crypto-path coverage, module provenance, and supported
operating environments need independent review before any compliance claim.
Source builds require a host and toolchain supported by `aws-lc-fips-sys`;
unsupported builds fail rather than falling back. No release platform matrix is established.

This distribution and `snowflake-connector-python` install into the **same**
`snowflake.connector` Python namespace. They are alternatives, not add-ons:
if both distribution names are installed, importing either now fails with an
error asking you to uninstall both and reinstall only one. The Python wrapper
also checks the native bridge's `fips-tls` build-feature indicator against its
distribution profile to reject mismatched shared files. This indicator shows
the selected build feature, **not** FIPS validation or certification.
The candidate has no runtime `cryptography` dependency. `private_key` accepts
DER bytes or base64-encoded PEM/DER strings, and rejects PyCA `RSAPrivateKey`
objects with `ProgrammingError`. The ordinary distribution still supports them.

Build from the repository checkout (Python 3.11+, Rust, C compiler, Cython,
setuptools, and Hatchling required):

```sh
cd python/fips-candidate
python -m build --sdist
python -m build --wheel
```

The sdist contains its own Rust workspace sources and lockfile. To build a wheel
from the archive, extract it and run `python -m build --wheel` inside the
extracted package root. A wheel build always compiles a fresh, interpreter-tagged
FIPS-TLS native bridge from this package's `Cargo.toml`; setting
`SKIP_CORE_BUILD` to a true value fails rather than packaging another build's
bridge. Editable installs are intentionally unavailable, because they could
import the ordinary distribution's profile from a shared checkout.

Before adding a release workflow, establish validated module/version/platform
provenance, check the remaining cryptographic paths and approved environments,
then add separate candidate-only wheel and sdist lanes for supported targets
with artifact-level checks. Do not reuse ordinary prebuilt native artifacts.
