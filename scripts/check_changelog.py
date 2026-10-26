#!/usr/bin/env python3
"""File-based changelog check.

In-progress notes are one fragment per pull request under
``<component>/changelog/unreleased/``. The file name is ``<slug>.md`` and
the slug does not start with a number, so two pull requests do not have to
claim the next id. The file starts with ``category:`` metadata: breaking,
feature, change, bugfix, dependency, or internal.

Notes that already shipped stay in ``<component>/CHANGELOG.md`` under their
``## <version>`` heading until a release is cut. A pull request must not
edit those sections. Cutting a release moves fragments from ``unreleased/``
into a new ``<component>/changelog/<version>/`` directory. A version
directory that already exists on the comparison base is frozen.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

# Suffix on a fragment file, then the section label rendered into CHANGELOG.md.
# The first six are the sections new entries use. The last three are labels
# that already shipped in component changelogs.
CATEGORIES: dict[str, str] = {
    "breaking": "Breaking changes",
    "feature": "New features",
    "change": "Changes",
    "bugfix": "Bug fixes",
    "dependency": "Dependencies",
    "internal": "Internal changes",
    "internal-improvements": "Internal improvements",
    "test-improvements": "Test improvements",
    "known-issues": "Known issues",
}

_LABEL_TO_CATEGORY = {label: key for key, label in CATEGORIES.items()}

# New notes use these six. The other CATEGORIES keys are labels that already
# shipped; a version folder may still contain them.
_RELEASE_NOTE_CATEGORY_ORDER = (
    "breaking",
    "feature",
    "change",
    "bugfix",
    "dependency",
    "internal",
)
RELEASE_NOTE_CATEGORIES: frozenset[str] = frozenset(_RELEASE_NOTE_CATEGORY_ORDER)
# Test-only upcoming notes are not release notes.
OMITTED_FROM_UNRELEASED: frozenset[str] = frozenset({"test-improvements"})
_CATEGORY_ON_MIGRATE = {"internal-improvements": "internal"}

_VERSION_RE = re.compile(
    r"^(?P<prefix>v)?(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)"
    r"(?P<pre>[-+][0-9A-Za-z.-]+)?$"
)
_VERSION_HEADING_RE = re.compile(
    r"^(?P<version>v?\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)"
    r"(?:\s+\((?P<date>[^)]*)\))?\s*$"
)
_SLUG_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
_LABEL_RE = re.compile(r"^([A-Za-z][A-Za-z ]*):$")
_UPCOMING_POINTER = "In-progress notes are files under `changelog/unreleased/`."
# The Node.js changelog is still a hand-written draft without fragment files.
_SKIPPED_COMPONENTS: frozenset[str] = frozenset({"nodejs"})


@dataclass(frozen=True)
class DiffEntry:
    status: str
    path: str
    old_path: str | None


@dataclass
class LegacySection:
    """One ``##`` block of a monolithic CHANGELOG.md.

    ``blocks`` keeps repeated section labels in the order they appear.
    ``v4.0.0-rc3`` has two ``Bug fixes:`` groups, and merging them would
    move the second group up next to the first.
    """

    version: str | None
    date: str | None
    blocks: list[tuple[str, list[str]]]


def version_sort_key(name: str) -> tuple:
    """Newest release first. A final release sorts ahead of its prereleases."""
    match = _VERSION_RE.match(name)
    if match is None:
        return (0, 0, 0, (0,))
    pre = match.group("pre") or ""
    if pre:
        parts: list[tuple] = []
        for piece in re.split(r"[.-]", pre[1:]):
            if piece.isdigit():
                parts.append((0, int(piece)))
            else:
                parts.append((1, piece))
        pre_key: tuple = (0, tuple(parts))
    else:
        pre_key = (1,)
    return (
        int(match.group("major")),
        int(match.group("minor")),
        int(match.group("patch")),
        pre_key,
    )


def parse_fragment_slug(filename: str) -> str | None:
    if not filename.endswith(".md"):
        return None
    slug = filename[: -len(".md")]
    if _SLUG_RE.match(slug) is None or _slug_has_numeric_id(slug):
        return None
    return slug


def fragment_text(category: str, body: str) -> str:
    """Fragment file: category metadata, then the changelog bullets."""
    if not body.endswith("\n"):
        body += "\n"
    return f"---\ncategory: {category}\n---\n\n{body}"


def split_fragment(text: str) -> tuple[str | None, str, list[str]]:
    """Return ``(category, bullet body, errors)`` for one fragment file."""
    if not text.startswith("---\n"):
        return None, text, ["fragment must start with category metadata"]
    rest = text[len("---\n") :]
    end = rest.find("\n---\n")
    if end < 0:
        return None, text, ["fragment metadata is missing its closing fence"]
    header = rest[:end]
    body = rest[end + len("\n---\n") :]
    if body.startswith("\n"):
        body = body[1:]
    errors: list[str] = []
    category: str | None = None
    for line in header.splitlines():
        if not line.strip():
            continue
        if not line.startswith("category:"):
            errors.append("only a category field is allowed in fragment metadata")
            continue
        value = line[len("category:") :].strip()
        if category is not None:
            errors.append("category is set more than once")
        elif value not in CATEGORIES:
            errors.append(f"unknown category {value!r}")
        else:
            category = value
    if category is None and not errors:
        errors.append("fragment metadata needs a category")
    return category, body, errors


def bullet_lines(text: str) -> list[str]:
    """Parent bullets and the indented continuation lines that belong to them."""
    _category, body, errors = split_fragment(text)
    source = body if not errors else text
    lines: list[str] = []
    for line in source.splitlines():
        if line.startswith("- "):
            lines.append(line)
        elif lines and line.startswith((" ", "\t")):
            lines.append(line)
    return lines


def bullet_words(bullet: str) -> list[str]:
    text = bullet[2:] if bullet.startswith("- ") else bullet
    return re.findall(r"[A-Za-z0-9]+", text.lower())


def slug_from_words(words: list[str], count: int) -> str:
    slug = "-".join(words[:count])
    return slug or "entry"


def _slug_has_numeric_id(slug: str) -> bool:
    return slug.split("-", 1)[0].isdigit()


def _slug_text(bullet: str) -> str:
    return bullet.split("\n", 1)[0]


def _unique_slug(words: list[str], taken: set[str]) -> str:
    limit = max(len(words), 1)
    chosen = ""
    for count in range(min(4, limit), limit + 1):
        candidate = slug_from_words(words, count)
        if not _slug_has_numeric_id(candidate) and candidate not in taken:
            chosen = candidate
            break
    if not chosen:
        chosen = slug_from_words(words, limit) or "entry"
        if _slug_has_numeric_id(chosen):
            chosen = f"entry-{chosen}"
    candidate = chosen
    suffix = 2
    while candidate in taken:
        candidate = f"{chosen}-{suffix}"
        suffix += 1
    return candidate


def assign_slugs(bullets: list[str]) -> list[str]:
    """Pick a slug from each bullet's own words.

    Identical bullets share a slug. Different bullets grow the slug until
    the names differ. A slug that is still taken gains a numeric suffix
    after the first word, so two short notes cannot overwrite one file.
    """
    taken: set[str] = set()
    by_text: dict[str, str] = {}
    slugs: list[str] = []
    for bullet in bullets:
        existing = by_text.get(bullet)
        if existing is not None:
            slugs.append(existing)
            continue
        slug = _unique_slug(bullet_words(_slug_text(bullet)), taken)
        taken.add(slug)
        by_text[bullet] = slug
        slugs.append(slug)
    return slugs


def discover_components(root: Path) -> list[Path]:
    found: list[Path] = []
    for child in sorted(root.iterdir()):
        if not child.is_dir() or child.is_symlink() or child.name in _SKIPPED_COMPONENTS:
            continue
        if (child / "CHANGELOG.md").is_file() or (child / "changelog").is_dir():
            found.append(child)
    return found


def parse_legacy(text: str, path: str) -> tuple[list[LegacySection], list[str]]:
    """Split a monolithic CHANGELOG.md into upcoming notes and version sections."""
    errors: list[str] = []
    releases: list[LegacySection] = []
    current: LegacySection | None = None
    block: list[str] | None = None

    for line_number, line in enumerate(text.splitlines(), 1):
        if line.startswith("## "):
            title = line[3:].strip()
            block = None
            if title == "Upcoming Release":
                current = LegacySection(version=None, date=None, blocks=[])
                releases.append(current)
                continue
            match = _VERSION_HEADING_RE.match(title)
            if match is None:
                errors.append(f"{path}:{line_number}: unrecognized changelog heading {title!r}")
                current = None
                continue
            date = match.group("date")
            current = LegacySection(
                version=match.group("version"),
                date=date.strip() if date else None,
                blocks=[],
            )
            releases.append(current)
            continue
        if line.startswith("# ") or line == _UPCOMING_POINTER:
            continue
        if not line.strip():
            continue
        label = _LABEL_RE.match(line)
        if label:
            name = label.group(1)
            category = _LABEL_TO_CATEGORY.get(name)
            if category is None:
                errors.append(f"{path}:{line_number}: unknown changelog section {name!r}")
                continue
            if current is None:
                errors.append(f"{path}:{line_number}: section {name!r} is outside a release")
                continue
            block = []
            current.blocks.append((category, block))
            continue
        if line.startswith("- "):
            if current is None or block is None:
                errors.append(f"{path}:{line_number}: bullet is outside a changelog section")
                continue
            block.append(line)
            continue
        if line.startswith((" ", "\t")):
            if current is None or block is None:
                errors.append(f"{path}:{line_number}: continuation line is outside a bullet")
                continue
            block.append(line)
            continue
        errors.append(f"{path}:{line_number}: expected a heading, a section label, or a bullet")
    return releases, errors


def upcoming_body(text: str) -> str:
    match = re.search(r"^## Upcoming Release\s*$", text, re.M)
    if match is None:
        return ""
    rest = text[match.end() :]
    nxt = re.search(r"^## ", rest, re.M)
    body = rest[: nxt.start()] if nxt else rest
    return body.strip()


def _upcoming_region(text: str) -> str:
    """Upcoming Release only. Released sections are frozen by byte compare."""
    if re.search(r"^## Upcoming Release\s*$", text, re.M) is None:
        return ""
    return "## Upcoming Release\n" + upcoming_body(text) + "\n"


def version_sections(text: str) -> dict[str, str]:
    """Map a released version to its ``##`` section, heading included."""
    matches = list(re.finditer(r"^## .+$", text, re.M))
    sections: dict[str, str] = {}
    for index, match in enumerate(matches):
        title = match.group(0)[3:].strip()
        parsed = _VERSION_HEADING_RE.match(title)
        if parsed is None:
            continue
        start = match.start()
        end = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        sections[parsed.group("version")] = text[start:end].strip()
    return sections


def _rendered_section(folder: Path) -> str:
    date, _errors = read_release_date(folder / ".release")
    heading = f"## {folder.name}"
    if date:
        heading += f" ({date})"
    body = "\n".join(_render_version_body(folder)).strip("\n")
    if not body:
        return heading
    return f"{heading}\n\n{body}"


def _fragments_by_category(folder: Path) -> dict[str, list[Path]]:
    grouped: dict[str, list[Path]] = {key: [] for key in CATEGORIES}
    if not folder.is_dir():
        return grouped
    for path in folder.iterdir():
        if not path.is_file():
            continue
        if parse_fragment_slug(path.name) is None:
            continue
        category, _body, errors = split_fragment(path.read_text(encoding="utf-8"))
        if category is None or errors:
            continue
        grouped[category].append(path)
    for paths in grouped.values():
        paths.sort(key=lambda item: item.name)
    return grouped


def _render_version_body(folder: Path) -> list[str]:
    """Group a release by section. Filenames do not carry a sequence number."""
    lines: list[str] = []
    for category, paths in _fragments_by_category(folder).items():
        if not paths:
            continue
        lines.append(f"{CATEGORIES[category]}:")
        lines.append("")
        for path in paths:
            lines.append(_fragment_body(path))
        lines.append("")
    return lines


def _fragment_body(path: Path) -> str:
    text = path.read_text(encoding="utf-8")
    _category, body, errors = split_fragment(text)
    if errors:
        body = text
    if body.endswith("\n"):
        body = body[:-1]
    return body


def read_release_date(path: Path) -> tuple[str | None, list[str]]:
    if not path.is_file():
        return None, []
    errors: list[str] = []
    date: str | None = None
    text = path.read_text(encoding="utf-8")
    if text and not text.endswith("\n"):
        errors.append(f"{path}: .release must end with a newline")
    for line_number, line in enumerate(text.splitlines(), 1):
        if not line.strip() or line.startswith("#"):
            continue
        if line.startswith("date:"):
            value = line[len("date:") :].strip()
            if not value:
                errors.append(f"{path}:{line_number}: date is empty")
            elif date is not None:
                errors.append(f"{path}:{line_number}: date is set more than once")
            else:
                date = value
            continue
        errors.append(f"{path}:{line_number}: only a date field is allowed in .release")
    return date, errors


def validate_fragment(path: Path, *, released: bool) -> list[str]:
    if parse_fragment_slug(path.name) is None:
        return [
            f"{path}: fragment file name must be <slug>.md "
            "and the slug does not start with a number"
        ]
    errors: list[str] = []
    text = path.read_text(encoding="utf-8")
    if not text.endswith("\n"):
        errors.append(f"{path}: fragment must end with a newline")
    category, body, meta_errors = split_fragment(text)
    errors.extend(f"{path}: {error}" for error in meta_errors)
    if category is not None and not released and category not in RELEASE_NOTE_CATEGORIES:
        allowed = ", ".join(_RELEASE_NOTE_CATEGORY_ORDER)
        errors.append(
            f"{path}: unreleased notes use {allowed}. "
            f"{category} is only valid in a released version folder."
        )
    if meta_errors:
        return errors
    body = body.strip("\n")
    if not body:
        errors.append(f"{path}: fragment is empty")
        return errors
    saw_bullet = False
    for line_number, line in enumerate(body.split("\n"), 1):
        if line.startswith("- "):
            saw_bullet = True
            if not line[2:].strip():
                errors.append(f"{path}:{line_number}: bullet is empty")
        elif line.startswith((" ", "\t")) or line == "":
            if line.startswith((" ", "\t")) and not saw_bullet:
                errors.append(f"{path}:{line_number}: continuation line is outside a bullet")
        else:
            errors.append(f"{path}:{line_number}: expected a markdown bullet starting with '- '")
    if not saw_bullet:
        errors.append(f"{path}: fragment has no changelog bullet")
    return errors


def validate_tree(root: Path) -> list[str]:
    errors: list[str] = []
    for component in discover_components(root):
        changelog = component / "changelog"
        rendered = component / "CHANGELOG.md"
        rel_changelog = changelog.relative_to(root)
        if not changelog.is_dir():
            errors.append(f"{rel_changelog}: missing changelog directory")
            continue
        if not rendered.is_file():
            errors.append(f"{rendered.relative_to(root)}: missing generated changelog")
            continue
        errors.extend(_validate_changelog_dir(root, changelog))
        actual = rendered.read_text(encoding="utf-8")
        if upcoming_body(actual) != _UPCOMING_POINTER:
            errors.append(
                f"{rendered.relative_to(root)}: Upcoming Release must be only "
                f"the pointer to {rel_changelog / 'unreleased'}."
            )
        sections = version_sections(actual)
        for path in sorted(changelog.iterdir()):
            if not path.is_dir() or _VERSION_RE.match(path.name) is None:
                continue
            expected = _rendered_section(path)
            found = sections.get(path.name)
            rel = path.relative_to(root)
            if found is None:
                errors.append(f"{rendered.relative_to(root)}: missing section for {rel}.")
            elif found != expected:
                errors.append(
                    f"{rendered.relative_to(root)}: section {path.name} does not match {rel}. "
                    "Run `python3 scripts/check_changelog.py --render`."
                )
    return errors


def _validate_changelog_dir(root: Path, changelog: Path) -> list[str]:
    errors: list[str] = []
    rel = changelog.relative_to(root)
    for path in sorted(changelog.iterdir()):
        if path.is_symlink():
            errors.append(f"{path.relative_to(root)}: changelog entries must be regular files")
            continue
        if path.is_file():
            errors.append(
                f"{path.relative_to(root)}: fragments belong in {rel / 'unreleased'} "
                "or a version folder"
            )
            continue
        if not path.is_dir():
            errors.append(f"{path.relative_to(root)}: unexpected changelog entry")
            continue
        if path.name == "unreleased":
            errors.extend(_validate_bucket(root, path, released=False))
        elif _VERSION_RE.match(path.name):
            errors.extend(_validate_bucket(root, path, released=True))
        else:
            errors.append(
                f"{path.relative_to(root)}: directory must be 'unreleased' or a version "
                "such as v1.2.3"
            )
    return errors


def _validate_bucket(root: Path, folder: Path, *, released: bool) -> list[str]:
    errors: list[str] = []
    for path in sorted(folder.iterdir()):
        rel = path.relative_to(root)
        if path.is_symlink() or not path.is_file():
            errors.append(f"{rel}: changelog folder must contain only fragment files")
            continue
        if path.name == ".gitkeep" and not released:
            continue
        if path.name == ".release" and released:
            _date, release_errors = read_release_date(path)
            errors.extend(release_errors)
            continue
        errors.extend(validate_fragment(path, released=released))
    return errors


def _git(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args],
        cwd=root,
        check=False,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )


def _git_ok(root: Path, *args: str) -> str:
    result = _git(root, *args)
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip()
        raise RuntimeError(f"git {' '.join(args)} failed: {detail}")
    return result.stdout


def parse_name_status(text: str) -> list[DiffEntry]:
    entries: list[DiffEntry] = []
    for line in text.splitlines():
        if not line:
            continue
        parts = line.split("\t")
        status = parts[0]
        if status.startswith(("R", "C")) and len(parts) == 3:
            entries.append(DiffEntry(status=status, path=parts[2], old_path=parts[1]))
        elif len(parts) == 2:
            entries.append(DiffEntry(status=status, path=parts[1], old_path=None))
        else:
            entries.append(DiffEntry(status=status, path=parts[-1], old_path=None))
    return entries


def _show_file(root: Path, base: str, rel: str) -> str | None:
    result = _git(root, "show", f"{base}:{rel}")
    if result.returncode != 0:
        return None
    return result.stdout


def _base_versions(root: Path, base: str, component: str) -> set[str]:
    result = _git(root, "ls-tree", "--name-only", f"{base}:{component}/changelog")
    if result.returncode != 0:
        return set()
    return {name for name in result.stdout.splitlines() if _VERSION_RE.match(name)}


def _diff_entries(root: Path, base: str) -> list[DiffEntry]:
    diff = _git_ok(
        root,
        "diff",
        "--name-status",
        "--find-renames=100%",
        "-M100%",
        "-l5000",
        base,
    )
    entries = parse_name_status(diff)
    others = _git_ok(root, "ls-files", "--others", "--exclude-standard")
    for path in others.splitlines():
        entries.append(DiffEntry(status="A", path=path, old_path=None))
    return entries


def _classify(path: str) -> tuple[str, str, str | None, str | None] | None:
    """Return (component, kind, version, filename) for a changelog path.

    kind is ``rendered``, ``unreleased``, ``version``, or ``other``.
    """
    parts = path.split("/")
    if len(parts) == 2 and parts[1] == "CHANGELOG.md":
        return parts[0], "rendered", None, None
    if len(parts) < 3 or parts[1] != "changelog":
        return None
    component = parts[0]
    if len(parts) == 3:
        return component, "other", None, parts[2]
    if len(parts) > 4:
        return component, "other", parts[2], "/".join(parts[3:])
    bucket, filename = parts[2], parts[3]
    if bucket == "unreleased":
        return component, "unreleased", None, filename
    if _VERSION_RE.match(bucket):
        return component, "version", bucket, filename
    return component, "other", bucket, filename


def _worktree_bullets(folder: Path) -> dict[str, list[str]]:
    bullets: dict[str, list[str]] = {}
    for category, paths in _fragments_by_category(folder).items():
        lines: list[str] = []
        for path in paths:
            lines.extend(bullet_lines(path.read_text(encoding="utf-8")))
        if lines:
            bullets[category] = lines
    return bullets


def validate_against_base(root: Path, base: str) -> list[str]:
    resolved = _git(root, "rev-parse", "--verify", f"{base}^{{commit}}")
    if resolved.returncode != 0:
        return [f"unknown git revision {base!r}"]
    base_commit = resolved.stdout.strip()

    errors: list[str] = []
    frozen: dict[str, set[str]] = {}
    base_releases: dict[str, list[LegacySection]] = {}
    for component in discover_components(root):
        name = component.name
        frozen[name] = _base_versions(root, base_commit, name)
        legacy_text = _show_file(root, base_commit, f"{name}/CHANGELOG.md")
        if legacy_text is None:
            base_releases[name] = []
            continue
        releases, parse_errors = parse_legacy(
            _upcoming_region(legacy_text), f"{name}/CHANGELOG.md"
        )
        errors.extend(parse_errors)
        base_releases[name] = releases

    new_version_entries: dict[tuple[str, str], list[DiffEntry]] = defaultdict(list)
    for entry in _diff_entries(root, base_commit):
        paths = [entry.path]
        if entry.old_path:
            paths.append(entry.old_path)
        for path in paths:
            classified = _classify(path)
            if classified is None:
                continue
            component, kind, version, _filename = classified
            if kind == "version" and version in frozen.get(component, set()):
                errors.append(
                    f"{component}/changelog/{version} is already released and cannot change. "
                    f"Add notes under {component}/changelog/unreleased/."
                )
                break
        classified = _classify(entry.path)
        if classified is None:
            continue
        component, kind, version, _filename = classified
        if kind == "version" and version not in frozen.get(component, set()):
            new_version_entries[(component, version)].append(entry)
        elif kind == "other" and path_is_changelog(entry.path):
            errors.append(
                f"{entry.path}: fragments belong in changelog/unreleased/ or a version folder"
            )

    for (component, version), entries in sorted(new_version_entries.items()):
        errors.extend(_validate_new_version(component, version, entries))
    errors.extend(_freeze_shipped_sections(root, base_commit))
    errors.extend(_upcoming_notes_preserved(root, base_releases, frozen))
    return errors


def _freeze_shipped_sections(root: Path, base_commit: str) -> list[str]:
    """Shipped ``## <version>`` prose stays byte-for-byte until a later migration."""
    errors: list[str] = []
    for component in discover_components(root):
        name = component.name
        base_text = _show_file(root, base_commit, f"{name}/CHANGELOG.md")
        if base_text is None:
            continue
        work_path = root / name / "CHANGELOG.md"
        if not work_path.is_file():
            errors.append(f"{name}/CHANGELOG.md: missing changelog")
            continue
        work_text = work_path.read_text(encoding="utf-8")
        base_sections = version_sections(base_text)
        work_sections = version_sections(work_text)
        for version, base_section in base_sections.items():
            if work_sections.get(version) != base_section:
                errors.append(
                    f"{name}/CHANGELOG.md: ## {version} is already released and cannot change. "
                    f"Add notes under {name}/changelog/unreleased/."
                )
        for version in work_sections:
            if version in base_sections:
                continue
            folder = root / name / "changelog" / version
            if not folder.is_dir():
                errors.append(
                    f"{name}/CHANGELOG.md: ## {version} is new and needs a "
                    f"{name}/changelog/{version}/ folder moved from unreleased/."
                )
    return errors


def path_is_changelog(path: str) -> bool:
    parts = path.split("/")
    return len(parts) >= 2 and parts[1] == "changelog"


def _validate_new_version(
    component: str,
    _version: str,
    entries: list[DiffEntry],
) -> list[str]:
    errors: list[str] = []
    for entry in entries:
        filename = entry.path.split("/")[-1]
        unreleased = f"{component}/changelog/unreleased/{filename}"
        if filename == ".release" and entry.status == "A" and entry.old_path is None:
            continue
        if entry.status == "R100" and entry.old_path == unreleased:
            continue
        errors.append(
            f"{entry.path}: a new release folder is created by moving fragment files "
            f"unchanged from {component}/changelog/unreleased/."
        )
    return errors


def _upcoming_notes_preserved(
    root: Path,
    base_releases: dict[str, list[LegacySection]],
    frozen: dict[str, set[str]],
) -> list[str]:
    errors: list[str] = []
    for component, releases in base_releases.items():
        upcoming = next((item for item in releases if item.version is None), None)
        if upcoming is None:
            continue
        required: list[str] = []
        for category, lines in upcoming.blocks:
            if category in OMITTED_FROM_UNRELEASED:
                continue
            required.extend(lines)
        if not required:
            continue
        found: list[str] = []
        changelog = root / component / "changelog"
        if changelog.is_dir():
            frozen_versions = frozen.get(component, set())
            for folder in changelog.iterdir():
                if not folder.is_dir() or folder.name in frozen_versions:
                    continue
                if folder.name != "unreleased" and _VERSION_RE.match(folder.name) is None:
                    continue
                for lines in _worktree_bullets(folder).values():
                    found.extend(lines)
        missing = list(required)
        for line in found:
            if line in missing:
                missing.remove(line)
        if missing:
            errors.append(
                f"{component}/CHANGELOG.md: Upcoming Release notes must be kept as "
                f"fragment files under {component}/changelog/. "
                f"Missing: {missing[0]}"
            )
    return errors


def validate(root: Path, base: str | None = None) -> list[str]:
    errors = validate_tree(root)
    if base is not None:
        errors.extend(validate_against_base(root, base))
    return errors


def changelog_keeping_released(original: str) -> str:
    """Replace Upcoming Release with the pointer and leave shipped sections in place."""
    match = re.search(r"^## (?:v?\d+\.\d+\.\d+).*$", original, re.M)
    header = "# Changelog\n\n## Upcoming Release\n\n" + _UPCOMING_POINTER + "\n\n"
    if match is None:
        return header.rstrip() + "\n"
    released = original[match.start() :]
    if not released.endswith("\n"):
        released += "\n"
    return header + released


def render_preserving(component: Path, existing: str) -> str:
    """Insert version-folder sections and keep shipped prose that has no folder."""
    changelog = component / "changelog"
    folder_names: list[str] = []
    if changelog.is_dir():
        folder_names = [
            path.name
            for path in changelog.iterdir()
            if path.is_dir() and _VERSION_RE.match(path.name)
        ]
    folder_names.sort(key=version_sort_key, reverse=True)
    lines = [
        "# Changelog",
        "",
        "## Upcoming Release",
        "",
        _UPCOMING_POINTER,
        "",
    ]
    emitted: set[str] = set()
    for version in folder_names:
        lines.append(_rendered_section(changelog / version))
        lines.append("")
        emitted.add(version)
    for version, section in version_sections(existing).items():
        if version in emitted:
            continue
        lines.append(section)
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def render_changelogs(root: Path) -> None:
    """Refresh CHANGELOG.md from version folders without dropping shipped prose."""
    for component in discover_components(root):
        rendered = component / "CHANGELOG.md"
        if not rendered.is_file():
            continue
        existing = rendered.read_text(encoding="utf-8")
        rendered.write_text(render_preserving(component, existing), encoding="utf-8")


def materialize(root: Path) -> list[str]:
    """Move Upcoming Release bullets into unreleased fragments. Refuses to overwrite."""
    errors: list[str] = []
    for component in discover_components(root):
        dest = component / "changelog"
        source = component / "CHANGELOG.md"
        if dest.exists():
            errors.append(f"{dest.relative_to(root)} already exists")
            continue
        if not source.is_file():
            errors.append(f"{source.relative_to(root)}: missing changelog")
            continue
        original = source.read_text(encoding="utf-8")
        releases, parse_errors = parse_legacy(original, str(source.relative_to(root)))
        errors.extend(parse_errors)
        if parse_errors:
            continue
        _write_unreleased(dest, [item for item in releases if item.version is None])
        source.write_text(changelog_keeping_released(original), encoding="utf-8")
    return errors


def _group_bullets(lines: list[str]) -> list[str]:
    """Keep an indented continuation line with the parent bullet above it."""
    grouped: list[str] = []
    current: list[str] = []
    for line in lines:
        if line.startswith("- ") and current:
            grouped.append("\n".join(current))
            current = []
        current.append(line)
    if current:
        grouped.append("\n".join(current))
    return grouped


def _write_unreleased(dest: Path, releases: list[LegacySection]) -> None:
    bullets: list[str] = []
    categories: list[str] = []
    for release in releases:
        for category, lines in release.blocks:
            if category in OMITTED_FROM_UNRELEASED:
                continue
            category = _CATEGORY_ON_MIGRATE.get(category, category)
            for bullet in _group_bullets(lines):
                bullets.append(bullet)
                categories.append(category)
    folder = dest / "unreleased"
    if not bullets:
        folder.mkdir(parents=True, exist_ok=True)
        (folder / ".gitkeep").write_text("", encoding="utf-8")
        return
    for slug, category, bullet in zip(assign_slugs(bullets), categories, bullets):
        folder.mkdir(parents=True, exist_ok=True)
        (folder / f"{slug}.md").write_text(
            fragment_text(category, bullet + "\n"),
            encoding="utf-8",
        )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--base",
        help="git revision whose released changelog folders must not change",
    )
    parser.add_argument("--root", type=Path, default=Path("."))
    parser.add_argument(
        "--render",
        action="store_true",
        help="refresh CHANGELOG.md from version folders, keeping shipped sections",
    )
    parser.add_argument(
        "--materialize",
        action="store_true",
        help="move Upcoming Release bullets into changelog/unreleased/ fragment files",
    )
    args = parser.parse_args(argv)
    root = args.root.resolve()
    if args.materialize and args.render:
        print("pass only one of --materialize and --render", file=sys.stderr)
        return 2
    if args.materialize:
        errors = materialize(root)
    elif args.render:
        render_changelogs(root)
        errors = []
    else:
        errors = validate(root, base=args.base)
    for error in errors:
        print(error, file=sys.stderr)
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
