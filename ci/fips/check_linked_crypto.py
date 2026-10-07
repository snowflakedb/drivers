#!/usr/bin/env python3
"""Inspect release artifacts built with CARGO_PROFILE_RELEASE_STRIP=false.

Unix: inspect native symbols with nm and shared libraries with readelf/otool.
Windows: inspect dumpbin /dependents and the final crate's /MAP linker output.
The caller must supply the map from the artifact's exact link; CI forces a
fresh link with a unique /MAP path rather than reusing cached map evidence.
This checks native module linkage, not RustCrypto call sites or CMVP validation.
"""

import argparse
from pathlib import Path
import re
import subprocess
import sys
import tomllib


# Match public API entry points, not namespaces: AWS-LC itself retains private
# helpers such as EVP_MD_pctx_ops_init and OPENSSL_memcpy in unstripped builds.
# Do not use nm -g: the final cdylib link localizes genuine public C APIs too.
OPENSSL_APIS = {
    "AES_encrypt", "AES_decrypt", "AES_set_encrypt_key", "AES_set_decrypt_key",
    "BN_mod_exp", "BN_mod_exp_mont", "BN_mod_exp_mont_consttime",
    "CMAC_Init", "CMAC_Update", "CMAC_Final",
    "DES_set_key", "DES_set_key_unchecked", "DES_ecb_encrypt", "DES_ede3_cbc_encrypt",
    "DH_compute_key", "ECDH_compute_key", "ECDSA_sign", "ECDSA_verify",
    "EVP_EncryptInit_ex", "EVP_EncryptUpdate", "EVP_EncryptFinal_ex",
    "EVP_DecryptInit_ex", "EVP_DecryptUpdate", "EVP_DecryptFinal_ex",
    "EVP_CipherInit_ex", "EVP_CipherUpdate", "EVP_CipherFinal_ex",
    "EVP_Digest", "EVP_DigestInit_ex", "EVP_DigestUpdate", "EVP_DigestFinal_ex",
    "EVP_DigestSignInit", "EVP_DigestSign", "EVP_DigestSignFinal",
    "EVP_DigestVerifyInit", "EVP_DigestVerify", "EVP_DigestVerifyFinal",
    "EVP_CIPHER_CTX_new", "EVP_CIPHER_fetch", "EVP_MD_CTX_new", "EVP_MD_fetch",
    "EVP_PKEY_new", "EVP_PKEY_encrypt", "EVP_PKEY_decrypt", "EVP_PKEY_derive",
    "EVP_PKEY_sign", "EVP_PKEY_verify", "EVP_PKEY_keygen",
    "EVP_aes_128_cbc", "EVP_aes_256_cbc", "EVP_aes_128_gcm", "EVP_aes_256_gcm",
    "EVP_sha256", "EVP_sha384", "EVP_sha512",
    "HMAC", "HMAC_Init_ex", "HMAC_Update", "HMAC_Final",
    "MD5", "MD5_Init", "MD5_Update", "MD5_Final",
    "OPENSSL_init_crypto", "OPENSSL_init_ssl", "OPENSSL_malloc", "OPENSSL_free",
    "OpenSSL_version", "OpenSSL_version_num", "SSLeay", "SSLeay_version",
    "OSSL_PROVIDER_load", "OSSL_PROVIDER_try_load",
    "PKCS5_PBKDF2_HMAC", "PKCS5_PBKDF2_HMAC_SHA1",
    "RAND_bytes", "RAND_priv_bytes", "RAND_bytes_ex", "RAND_priv_bytes_ex",
    "RSA_new", "RSA_sign", "RSA_verify", "RSA_public_encrypt", "RSA_private_decrypt",
    "SHA1", "SHA1_Init", "SHA1_Update", "SHA1_Final",
    "SHA256", "SHA256_Init", "SHA256_Update", "SHA256_Final",
    "SHA512", "SHA512_Init", "SHA512_Update", "SHA512_Final",
    "SSL_CTX_new", "SSL_CTX_new_ex", "SSL_CTX_free", "SSL_new", "SSL_free",
    "SSL_connect", "SSL_accept", "SSL_read", "SSL_write", "SSL_library_init",
    "TLS_method", "TLS_client_method", "TLS_server_method", "d2i_X509",
}
MAX_OFFENDERS = 20


def locked_fips_prefix(lockfile: Path) -> str:
    with lockfile.open("rb") as source:
        packages = tomllib.load(source).get("package", [])
    versions = {p["version"] for p in packages if p["name"] == "aws-lc-fips-sys"}
    if len(versions) != 1:
        raise ValueError(f"{lockfile}: expected exactly one aws-lc-fips-sys version, got {sorted(versions)}")
    version = versions.pop()
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise ValueError(f"{lockfile}: unsupported aws-lc-fips-sys version {version!r}")
    return f"aws_lc_fips_{version.replace('.', '_')}_"


def normalize_symbol(symbol: str) -> str:
    # Mach-O's leading underscore and MSVC's import decoration are not part of
    # the library's symbol prefix. ELF symbol versions follow an @ suffix.
    return symbol.removeprefix("__imp_").lstrip("_").split("@", 1)[0]


def nm_symbols(output: str) -> dict[str, str]:
    symbols = {}
    for line in output.splitlines():
        match = re.fullmatch(r"\s*(?:[0-9a-fA-F]+\s+)?([a-zA-Z?])\s+(\S+)", line)
        if match:
            kind, name = match.groups()
            symbols[normalize_symbol(name)] = kind
    return symbols


def capped(values: list[str]) -> str:
    shown = ", ".join(values[:MAX_OFFENDERS])
    if len(values) > MAX_OFFENDERS:
        shown += f", ... ({len(values) - MAX_OFFENDERS} more)"
    return shown


def symbol_errors(symbols: dict[str, str], prefix: str, require_fips: bool = True) -> list[str]:
    errors = []
    if require_fips and not any(
        name.startswith(prefix) and kind in {"T", "t"}
        for name, kind in symbols.items()
    ):
        errors.append(f"no defined {prefix} symbols; retain release symbols (CARGO_PROFILE_RELEASE_STRIP=false)")
    offenders = sorted(
        name for name in symbols
        if re.match(r"aws_lc_\d+_", name)
        or name.startswith("ring_core_")
        or name.split(".", 1)[0] in OPENSSL_APIS
        or (re.match(r"aws_lc_fips_\d+_", name) and not name.startswith(prefix))
    )
    if offenders:
        errors.append(f"forbidden crypto symbols: {capped(offenders)}")
    return errors


def is_openssl_library(name: str) -> bool:
    basename = name.replace("\\", "/").rsplit("/", 1)[-1].lower()
    return bool(re.match(r"(?:lib)?(?:ssl|crypto)(?:[.\d-]|$)|(?:ssleay|libeay)\d", basename))


def unix_library_errors(output: str, platform: str) -> list[str]:
    if platform == "darwin":
        libraries = re.findall(r"^\s+(.+?)\s+\(compatibility version", output, re.MULTILINE)
    else:
        libraries = re.findall(r"\(NEEDED\).*?\[([^\]]+)\]", output)
    forbidden = sorted({name for name in libraries if is_openssl_library(name)})
    return [f"forbidden OpenSSL library dependencies: {capped(forbidden)}"] if forbidden else []


def windows_errors(dependencies: str, linker_map: str, prefix: str) -> list[str]:
    dlls = set(re.findall(r"^\s+([^\s]+\.dll)\s*$", dependencies.lower(), re.MULTILINE))
    errors = []
    expected_dll = f"{prefix}crypto.dll"
    if expected_dll not in dlls:
        errors.append(f"missing FIPS module import: {expected_dll}")
    forbidden_dlls = sorted(
        dll for dll in dlls
        if is_openssl_library(dll) or re.match(r"aws_lc_\d+_", dll)
        or (dll.startswith("aws_lc_fips_") and not dll.startswith(prefix))
    )
    if forbidden_dlls:
        errors.append(f"forbidden crypto DLL imports: {capped(forbidden_dlls)}")
    symbols = {
        normalize_symbol(match[1]): "T"
        for match in re.finditer(
            r"^\s*[0-9a-fA-F]{4}:[0-9a-fA-F]+\s+(\S+)\s+[0-9a-fA-F]{8,}\b",
            linker_map, re.MULTILINE,
        )
    }
    if not symbols:
        errors.append("no symbols parsed from MSVC linker map; supply the final crate's /MAP output")
    # Map entries include imports as well as definitions; dumpbin establishes
    # FIPS linkage, while the map exposes forbidden statically linked symbols.
    errors.extend(symbol_errors(symbols, prefix, require_fips=False))
    return errors


def run_tool(*command: str) -> str:
    result = subprocess.run(command, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if result.returncode:
        raise ValueError(
            f"{' '.join(command)} exited {result.returncode}: "
            f"{(result.stderr or result.stdout).strip()[:2000]}"
        )
    return result.stdout


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("artifacts", type=Path, nargs="+", help="unstripped release native artifacts")
    parser.add_argument(
        "--lockfile", type=Path, default=Path(__file__).resolve().parents[2] / "Cargo.lock",
    )
    parser.add_argument(
        "--linker-map", type=Path,
        help="Windows: cargo rustc /MAP output from this artifact's exact final link",
    )
    args = parser.parse_args()
    if sys.platform == "win32" and (args.linker_map is None or len(args.artifacts) != 1):
        parser.error("Windows requires one artifact and its --linker-map per invocation")
    if sys.platform != "win32" and args.linker_map is not None:
        parser.error("--linker-map is only used on Windows")
    try:
        prefix = locked_fips_prefix(args.lockfile)
    except (OSError, ValueError, KeyError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1

    failed = False
    for artifact in args.artifacts:
        try:
            if not artifact.is_file():
                raise ValueError("artifact is not a file")
            if sys.platform == "win32":
                errors = windows_errors(
                    run_tool("dumpbin", "/dependents", str(artifact)),
                    args.linker_map.read_text(encoding="utf-8", errors="replace"), prefix,
                )
            else:
                errors = symbol_errors(nm_symbols(run_tool("nm", "-a", str(artifact))), prefix)
                dependency_command = ("otool", "-L") if sys.platform == "darwin" else ("readelf", "-d")
                errors.extend(unix_library_errors(
                    run_tool(*dependency_command, str(artifact)), sys.platform,
                ))
        except (OSError, ValueError) as error:
            errors = [str(error)]
        if errors:
            failed = True
            print(f"FAIL {artifact}:\n  " + "\n  ".join(errors), file=sys.stderr)
        else:
            print(f"PASS {artifact}: {prefix} linked; no non-FIPS AWS-LC, ring or unprefixed OpenSSL found")
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
