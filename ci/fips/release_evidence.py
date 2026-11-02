#!/usr/bin/env python3
"""Write a FIPS candidate evidence bundle for one native library.

The bundle describes the file this job just built and inspected. Release
Engineering signing, notarization, and repackaging can change the bytes
customers download, so this bundle does not attest those published files.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tomllib

from check_linked_crypto import linked_libraries


SYS_CRATE = "aws-lc-fips-sys"
REPORTED_VERSION_PREFIX = "AWS-LC FIPS "
FORBIDDEN_CRATES = {
    "ring",
    "openssl",
    "openssl-sys",
    "openssl-src",
    "openssl-macros",
    "native-tls",
}
SBOM_STEM = "fips-candidate"
# cargo-cyclonedx 0.5.9 omits `.cdx` on an overridden name and then appends
# the format, so this value is written as fips-candidate.cdx.json.
SBOM_OVERRIDE = f"{SBOM_STEM}.cdx"


class EvidenceError(Exception):
    pass


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_module_identity(repo_root: Path) -> tuple[str, str]:
    cargo_toml = repo_root / "sf_core" / "Cargo.toml"
    tls_mod = repo_root / "sf_core" / "src" / "tls" / "mod.rs"
    with cargo_toml.open("rb") as source:
        dependency = tomllib.load(source).get("dependencies", {}).get(SYS_CRATE)
    if not isinstance(dependency, dict):
        raise EvidenceError(f"{cargo_toml}: {SYS_CRATE} is not a table dependency")
    pin = dependency.get("version")
    if not isinstance(pin, str) or not pin.startswith("="):
        raise EvidenceError(f"{cargo_toml}: {SYS_CRATE} must be an exact '=x.y.z' pin")
    sys_version = pin[1:]
    if not re.fullmatch(r"\d+\.\d+\.\d+", sys_version):
        raise EvidenceError(f"{cargo_toml}: unsupported {SYS_CRATE} pin {pin!r}")

    versions = re.findall(
        r'const EXPECTED_FIPS_MODULE_VERSION: &str = "([^"]+)";',
        tls_mod.read_text(encoding="utf-8"),
    )
    if len(versions) != 1:
        raise EvidenceError(
            f"{tls_mod}: expected one EXPECTED_FIPS_MODULE_VERSION, found {len(versions)}"
        )
    reported = versions[0]
    if not reported.startswith(REPORTED_VERSION_PREFIX) or not reported[len(REPORTED_VERSION_PREFIX):]:
        raise EvidenceError(
            f"{tls_mod}: module version {reported!r} is not 'AWS-LC FIPS <version>'"
        )
    return sys_version, reported


def iter_components(bom: dict) -> list[dict]:
    found = []

    def walk(value) -> None:
        if isinstance(value, dict) and value.get("type") in {
            "library", "application", "framework", "file",
        } and "name" in value:
            found.append(value)
            for child in value.get("components") or []:
                walk(child)
        elif isinstance(value, list):
            for item in value:
                walk(item)

    metadata = bom.get("metadata") or {}
    walk(metadata.get("component"))
    for component in bom.get("components") or []:
        walk(component)
    return found


def set_property(owner: dict, name: str, value: str) -> None:
    properties = owner.setdefault("properties", [])
    for prop in properties:
        if prop.get("name") == name:
            prop["value"] = value
            return
    properties.append({"name": name, "value": value})


def add_tool(metadata: dict, name: str, version: str) -> None:
    entry = {"type": "application", "name": name, "version": version}
    tools = metadata.get("tools")
    if tools is None:
        metadata["tools"] = [entry]
    elif isinstance(tools, list):
        tools.append(entry)
    elif isinstance(tools, dict):
        tools.setdefault("components", []).append(entry)
    else:
        raise EvidenceError("SBOM metadata.tools has an unexpected shape")


def sbom_errors(
    bom: dict,
    sys_version: str,
    reported_version: str,
    compiled: set[str] | None = None,
) -> list[str]:
    components = iter_components(bom)
    errors = []
    module_number = reported_version[len(REPORTED_VERSION_PREFIX):]
    fips_components = [
        component for component in components
        if component.get("name") == SYS_CRATE and component.get("scope") != "excluded"
    ]
    versions = {component.get("version") for component in fips_components}
    if len(fips_components) != 1 or versions != {sys_version}:
        errors.append(
            f"expected one runtime {SYS_CRATE} {sys_version}, found versions {sorted(versions)}"
        )
    module_components = [
        component for component in components
        if component.get("name") == "AWS-LC FIPS" and component.get("version") == module_number
    ]
    if len(module_components) != 1:
        errors.append(f"expected one AWS-LC FIPS {module_number} component")
    else:
        reported = next(
            (
                prop.get("value")
                for prop in module_components[0].get("properties") or []
                if prop.get("name") == "snowflake:reported-version"
            ),
            None,
        )
        if reported != reported_version:
            errors.append(f"module component reports {reported!r}, expected {reported_version!r}")
    forbidden = sorted({
        component.get("name")
        for component in components
        if component.get("name") in FORBIDDEN_CRATES
    })
    # `None` means the caller did not supply the compile graph, so every
    # forbidden name is treated as linked. A set is the normal/build cargo tree.
    if compiled is None:
        linked = forbidden
        inert = []
    else:
        linked = [name for name in forbidden if name in compiled]
        inert = [name for name in forbidden if name not in compiled]
    if linked:
        errors.append(f"forbidden crates in SBOM: {', '.join(linked)}")
    properties = {
        prop.get("name"): prop.get("value")
        for prop in (bom.get("metadata") or {}).get("properties") or []
    }
    for name in inert:
        if not properties.get(f"snowflake:{name}"):
            errors.append(f"{name} is present without a snowflake:{name} explanation")
    if any(component.get("name") == "aws-lc-sys" for component in components):
        note = next(
            (
                prop.get("value")
                for prop in (bom.get("metadata") or {}).get("properties") or []
                if prop.get("name") == "snowflake:aws-lc-sys"
            ),
            None,
        )
        if not note:
            errors.append("aws-lc-sys is present without a snowflake:aws-lc-sys explanation")
    return errors


def annotate_sbom(
    bom: dict,
    sys_version: str,
    reported_version: str,
    *,
    subject_name: str,
    subject_sha256: str,
    tools: list[tuple[str, str]],
    compiled: set[str] | None = None,
) -> dict:
    components = bom.setdefault("components", [])
    module_number = reported_version[len(REPORTED_VERSION_PREFIX):]
    existing = [
        component for component in components
        if component.get("name") == "AWS-LC FIPS" and component.get("version") == module_number
    ]
    module = existing[0] if existing else {"type": "library", "name": "AWS-LC FIPS", "version": module_number}
    if not existing:
        components.append(module)
    module["description"] = (
        f"Cryptographic module selected by {SYS_CRATE} {sys_version}. "
        "Graph membership is not native linkage evidence."
    )
    set_property(module, "snowflake:reported-version", reported_version)
    set_property(module, "snowflake:sys-crate", SYS_CRATE)
    set_property(module, "snowflake:sys-crate-version", sys_version)

    metadata = bom.setdefault("metadata", {})
    set_property(metadata, "snowflake:subject-name", subject_name)
    set_property(metadata, "snowflake:subject-sha256", subject_sha256)
    set_property(metadata, "snowflake:stage", "gha-candidate")
    set_property(metadata, "snowflake:linkage-report", "linkage.txt")
    if any(component.get("name") == "aws-lc-sys" for component in iter_components(bom)):
        set_property(
            metadata,
            "snowflake:aws-lc-sys",
            "Present in the Cargo graph. linkage.txt is the check of the native file.",
        )
    if compiled is not None:
        present = {
            component.get("name")
            for component in iter_components(bom)
            if component.get("name") in FORBIDDEN_CRATES
        }
        for name in sorted(present - compiled):
            set_property(
                metadata,
                f"snowflake:{name}",
                "cargo metadata lists this crate. The normal and build tree for "
                "this package does not include it. linkage.txt checks the native file.",
            )
    for name, version in tools:
        add_tool(metadata, name, version)
    errors = sbom_errors(bom, sys_version, reported_version, compiled)
    if errors:
        raise EvidenceError("; ".join(errors))
    return bom


def cyclonedx_command(manifest: Path, features: str, target: str) -> list[str]:
    command = [
        "cargo", "cyclonedx",
        "--format", "json",
        "--spec-version", "1.5",
        "--all",
        "--features", features,
        "--manifest-path", str(manifest),
        "--override-filename", SBOM_OVERRIDE,
        "--quiet",
    ]
    if target:
        command.extend(["--target", target])
    return command


def generated_sbom(manifest: Path) -> Path:
    direct = manifest.parent / f"{SBOM_STEM}.cdx.json"
    if direct.is_file():
        return direct
    matches = sorted(manifest.parent.glob(f"{SBOM_STEM}*.cdx.json"))
    if len(matches) == 1:
        return matches[0]
    raise EvidenceError(
        f"expected one {SBOM_STEM}.cdx.json beside {manifest}, found {[str(match) for match in matches]}"
    )


def discard_generated_sboms(root: Path) -> None:
    skipped = {"target", ".git", "node_modules"}
    for directory, dirnames, filenames in os.walk(root):
        dirnames[:] = [name for name in dirnames if name not in skipped]
        for filename in filenames:
            if filename.startswith(SBOM_STEM) and filename.endswith(".cdx.json"):
                Path(directory, filename).unlink()


def compiled_crate_names(package: str, features: str, target: str) -> set[str]:
    command = [
        "cargo", "tree",
        "-p", package,
        "--edges", "normal,build",
        "--prefix", "none",
        "--format", "{p}",
        "--features", features,
        "--locked",
    ]
    if target:
        command.extend(["--target", target])
    result = subprocess.run(command, text=True, capture_output=True, encoding="utf-8", errors="replace")
    if result.returncode:
        output = (result.stdout or "") + (result.stderr or "")
        raise EvidenceError(f"{' '.join(command)} exited {result.returncode}:\n{output[-4000:]}")
    names = {line.split()[0] for line in result.stdout.splitlines() if line.split()}
    if not names:
        raise EvidenceError(f"cargo tree listed no crates for {package}")
    return names


def capture(command: list[str]) -> str:
    result = subprocess.run(command, text=True, capture_output=True, encoding="utf-8", errors="replace")
    output = (result.stdout or "") + (result.stderr or "")
    if result.returncode:
        raise EvidenceError(f"{' '.join(command)} exited {result.returncode}:\n{output[-4000:]}")
    return output


def module_filename(sys_version: str, platform: str) -> str | None:
    stem = f"aws_lc_fips_{sys_version.replace('.', '_')}_crypto"
    if platform == "win32":
        return f"{stem}.dll"
    if platform == "darwin":
        return f"lib{stem}.dylib"
    return None


def module_search_dirs(artifact: Path, repo_root: Path, target: str) -> list[Path]:
    # These directories hold a module copied beside the library. The build
    # script's own output lives under release/build and is resolved separately.
    candidates = [artifact.parent]
    if target:
        release = repo_root / "target" / target / "release"
        candidates.extend((release, release / "deps"))
    directories = []
    seen = set()
    for directory in candidates:
        if not directory.is_dir():
            continue
        key = directory.resolve()
        if key in seen:
            continue
        seen.add(key)
        directories.append(directory)
    return directories


def first_named_file(name: str, directories: list[Path]) -> Path | None:
    for directory in directories:
        matches = [
            path for path in sorted(directory.iterdir())
            if path.is_file() and path.name.lower() == name.lower()
        ]
        if len(matches) > 1:
            found = ", ".join(path.name for path in matches)
            raise EvidenceError(f"more than one {name} in {directory}: {found}")
        if matches:
            return matches[0]
    return None


def build_output_freshness(path: Path, build_dir: Path) -> float:
    for parent in path.parents:
        stamp = parent / "invoked.timestamp"
        if stamp.is_file():
            return stamp.stat().st_mtime
        if parent == build_dir:
            break
    return path.stat().st_mtime


def built_module(build_dir: Path, name: str) -> Path | None:
    # aws-lc-fips-sys leaves the module in its build-script output. A cached
    # target directory can keep an older copy of the same filename. Cargo
    # writes invoked.timestamp for the build script this link ran.
    if not build_dir.is_dir():
        return None
    matches = [
        path for path in build_dir.rglob(name)
        if path.is_file() and path.name.lower() == name.lower()
    ]
    if not matches:
        return None
    if len(matches) == 1:
        return matches[0]
    ordered = sorted(matches, key=lambda path: build_output_freshness(path, build_dir), reverse=True)
    if build_output_freshness(ordered[0], build_dir) == build_output_freshness(ordered[1], build_dir):
        found = ", ".join(str(path) for path in ordered)
        raise EvidenceError(f"more than one {name} under {build_dir}: {found}")
    return ordered[0]


def resolve_module_file(name: str, artifact: Path, repo_root: Path, target: str) -> Path:
    directories = module_search_dirs(artifact, repo_root, target)
    found = first_named_file(name, directories)
    if found is not None:
        return found
    build_dir = None
    if target:
        build_dir = repo_root / "target" / target / "release" / "build"
        built = built_module(build_dir, name)
        if built is not None:
            return built
    places = [str(directory) for directory in directories]
    if build_dir is not None:
        places.append(str(build_dir))
    raise EvidenceError(f"{name} not found in {', '.join(places)}")


def windows_dll_imports(dumpbin_output: str) -> list[str]:
    return re.findall(r"^\s+(\S+\.dll)\s*$", dumpbin_output, re.MULTILINE | re.IGNORECASE)


def optional_rust_wrapper(crypto: Path) -> list[Path]:
    # rust_wrapper.c is linked into the crate. A sibling DLL is recorded when
    # this build emits one.
    suffix = "_crypto.dll"
    if not crypto.name.lower().endswith(suffix):
        return []
    wrapper_name = crypto.name[:-len(suffix)] + "_rust_wrapper.dll"
    matches = [
        path for path in sorted(crypto.parent.iterdir())
        if path.is_file() and path.name.lower() == wrapper_name.lower()
    ]
    if len(matches) > 1:
        raise EvidenceError(f"more than one {wrapper_name} in {crypto.parent}")
    return matches


def windows_companions(
    artifact: Path,
    repo_root: Path,
    target: str,
    sys_version: str,
    dumpbin_output: str,
) -> list[Path]:
    expected = module_filename(sys_version, "win32")
    imported = next(
        (name for name in windows_dll_imports(dumpbin_output) if name.lower() == expected.lower()),
        None,
    )
    if imported is None:
        raise EvidenceError(f"dumpbin /dependents does not import {expected}")
    crypto = resolve_module_file(expected, artifact, repo_root, target)
    return [crypto, *optional_rust_wrapper(crypto)]


def darwin_companions(
    artifact: Path,
    repo_root: Path,
    target: str,
    sys_version: str,
    otool_output: str,
) -> list[Path]:
    expected = module_filename(sys_version, "darwin")
    linked = [
        raw for raw in linked_libraries(otool_output, "darwin")
        if Path(raw).name == expected
    ]
    if len(linked) != 1:
        raise EvidenceError(f"otool -L does not link exactly one {expected}")
    named = Path(linked[0])
    if named.is_file():
        return [named]
    return [resolve_module_file(expected, artifact, repo_root, target)]


def companion_files(
    artifact: Path,
    repo_root: Path,
    target: str,
    sys_version: str,
    dumpbin_output: str = "",
) -> list[Path]:
    if sys.platform == "win32":
        return windows_companions(artifact, repo_root, target, sys_version, dumpbin_output)
    if sys.platform == "darwin":
        otool_output = capture(["otool", "-L", str(artifact)])
        return darwin_companions(artifact, repo_root, target, sys_version, otool_output)
    return []


def repo_relative(path: Path, repo_root: Path) -> str:
    resolved = path.resolve()
    try:
        return resolved.relative_to(repo_root.resolve()).as_posix()
    except ValueError:
        return resolved.as_posix()


def cyclonedx_pin_matches(output: str, expected: str) -> bool:
    return re.search(rf"(?<![\d.]){re.escape(expected)}(?![\d.])", output) is not None


def toolchain_record(cyclonedx_version: str) -> tuple[dict, list[tuple[str, str]]]:
    rustc = capture(["rustc", "-vV"])
    cargo = capture(["cargo", "--version"]).strip()
    cyclonedx = capture(["cargo", "cyclonedx", "--version"]).strip()
    if not cyclonedx_pin_matches(cyclonedx, cyclonedx_version):
        raise EvidenceError(f"cargo-cyclonedx {cyclonedx!r} does not match pin {cyclonedx_version}")
    release = next(
        (line.removeprefix("release: ") for line in rustc.splitlines() if line.startswith("release: ")),
        "",
    )
    host = next(
        (line.removeprefix("host: ") for line in rustc.splitlines() if line.startswith("host: ")),
        "",
    )
    record = {
        "rustc": rustc.strip(),
        "cargo": cargo,
        "cargo_cyclonedx": cyclonedx,
        "cc": os.environ.get("CC", ""),
        "cxx": os.environ.get("CXX", ""),
    }
    if host:
        record["host"] = host
    return record, [("rustc", release), ("cargo", cargo), ("cargo-cyclonedx", cyclonedx_version)]


def write_bundle(args: argparse.Namespace) -> None:
    repo_root = args.repo_root.resolve()
    artifact = args.artifact.resolve()
    output_dir = args.output_dir.resolve()
    manifest_path = args.manifest_path.resolve()
    if not artifact.is_file():
        raise EvidenceError(f"artifact is not a file: {artifact}")
    if sys.platform == "win32" and (args.linker_map is None or not args.linker_map.is_file()):
        raise EvidenceError("Windows evidence requires the artifact's linker map")
    sys_version, reported_version = read_module_identity(repo_root)
    lockfile = repo_root / "Cargo.lock"
    lock_digest = sha256_file(lockfile)
    output_dir.mkdir(parents=True, exist_ok=True)

    target = args.target or os.environ.get("CARGO_BUILD_TARGET", "")
    try:
        capture(cyclonedx_command(manifest_path, args.features, target))
        generated = generated_sbom(manifest_path)
        bom = json.loads(generated.read_text(encoding="utf-8"))
        if sha256_file(lockfile) != lock_digest:
            raise EvidenceError("cargo cyclonedx changed Cargo.lock")
    finally:
        discard_generated_sboms(repo_root)

    subject_sha256 = sha256_file(artifact)
    toolchain, tools = toolchain_record(args.cyclonedx_version)
    annotate_sbom(
        bom,
        sys_version,
        reported_version,
        subject_name=artifact.name,
        subject_sha256=subject_sha256,
        tools=tools,
        compiled=compiled_crate_names(args.package, args.features, target),
    )
    sbom_path = output_dir / "sbom.cdx.json"
    sbom_path.write_text(json.dumps(bom, indent=2) + "\n", encoding="utf-8")

    linkage_command = [
        sys.executable,
        str(Path(__file__).with_name("check_linked_crypto.py")),
        "--lockfile", str(lockfile),
        str(artifact),
    ]
    if args.linker_map is not None:
        linkage_command.extend(["--linker-map", str(args.linker_map)])
    linkage = subprocess.run(
        linkage_command, text=True, capture_output=True, encoding="utf-8", errors="replace",
    )
    linkage_text = (linkage.stdout or "") + (linkage.stderr or "")
    (output_dir / "linkage.txt").write_text(linkage_text, encoding="utf-8")
    dumpbin_output = ""
    if sys.platform == "win32":
        dumpbin_output = capture(["dumpbin", "/dependents", str(artifact)])
        (output_dir / "dumpbin.txt").write_text(dumpbin_output, encoding="utf-8")
        shutil.copyfile(args.linker_map, output_dir / "linker-map.txt")
    if linkage.returncode:
        raise EvidenceError(f"linkage check failed:\n{linkage_text}")

    companions = [
        {
            "name": path.name,
            "path": repo_relative(path, repo_root),
            "sha256": sha256_file(path),
        }
        for path in companion_files(artifact, repo_root, target, sys_version, dumpbin_output)
    ]
    hashes = {
        "artifact": {"name": artifact.name, "sha256": subject_sha256},
        "sbom": {"name": sbom_path.name, "sha256": sha256_file(sbom_path)},
        "companions": companions,
    }
    (output_dir / "hashes.json").write_text(json.dumps(hashes, indent=2) + "\n", encoding="utf-8")
    (output_dir / "toolchain.json").write_text(json.dumps(toolchain, indent=2) + "\n", encoding="utf-8")
    manifest = {
        "schema_version": 1,
        "stage": "gha-candidate",
        "covers_releng_published_bytes": False,
        "package": args.package,
        "features": args.features,
        "target": target,
        "artifact_sha256": subject_sha256,
        "sbom_sha256": hashes["sbom"]["sha256"],
        "module": {
            "crate": SYS_CRATE,
            "crate_version": sys_version,
            "reported_version": reported_version,
        },
        "self_check": {
            "status": "not-run",
            "reason": (
                "Binding status APIs are withheld, so the packaged artifact "
                "cannot report fips_build_enabled."
            ),
        },
        "source_sha": os.environ.get("GITHUB_SHA", ""),
    }
    (output_dir / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package", required=True)
    parser.add_argument("--manifest-path", type=Path, required=True)
    parser.add_argument("--artifact", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--features", default="fips")
    parser.add_argument("--target", default="")
    parser.add_argument("--linker-map", type=Path)
    parser.add_argument("--repo-root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--cyclonedx-version", default="0.5.9")
    args = parser.parse_args()
    try:
        write_bundle(args)
    except (OSError, EvidenceError, json.JSONDecodeError, subprocess.SubprocessError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    print(f"PASS {args.package}: wrote {args.output_dir}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
