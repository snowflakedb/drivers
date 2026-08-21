"""Per-driver SELECT fetch/bind variants. Suites pick suffixes from this catalog."""

from dataclasses import dataclass

import pytest


@dataclass(frozen=True)
class Variant:
    suffix: str
    fetch_mode: str = "fetchmany"
    bind_mode: str = "char"


VARIANTS: dict[str, tuple[Variant, ...]] = {
    "python": (
        Variant(""),
        Variant("_fetchall", fetch_mode="fetchall"),
        Variant("_fetchone", fetch_mode="fetchone"),
        Variant("_pandas", fetch_mode="pandas"),
        Variant("_arrow_batches", fetch_mode="arrow_batches"),
    ),
    "jdbc": (Variant(""),),
    "odbc": (
        Variant(""),
        Variant("_default", bind_mode="default"),
    ),
    "core": (Variant(""),),
}


def cases(sizes, suffixes, *, infix=""):
    """Pytest params of (row_count, name, fetch_mode) marked with supported_drivers.

    `infix` is inserted between the size label and the variant suffix
    (e.g. infix="_arrow" → `10k_arrow_fetchall`).
    """
    unique_suffixes = list(dict.fromkeys(s for ss in suffixes.values() for s in ss))
    return [
        pytest.param(
            row_count,
            f"{label}{infix}{suffix}",
            _variant(suffix).fetch_mode,
            id=f"{label}{infix}{suffix}",
            marks=pytest.mark.supported_drivers(*_drivers(suffix, suffixes)),
        )
        for row_count, label in sizes
        for suffix in unique_suffixes
    ]


def _variant(suffix: str) -> Variant:
    for variants in VARIANTS.values():
        for v in variants:
            if v.suffix == suffix:
                return v
    raise KeyError(f"No variant with suffix {suffix!r}")


def _drivers(suffix: str, suffixes) -> tuple[str, ...]:
    return tuple(driver for driver, ss in suffixes.items() if suffix in ss)
