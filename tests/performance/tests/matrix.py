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
        Variant(""),  # default one is fetchmany
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


def cases(sizes, suffixes, *, infix="", types=None, id_suffix=""):
    """Pytest params marked with supported_drivers.

    Without `types`: (row_count, name, fetch_mode, bind_mode).
    With `types`: (row_count, dtype, name, fetch_mode, bind_mode).

    `infix` is inserted between the size label and the variant suffix
    (e.g. infix="_arrow" → `10k_arrow_fetchall`).

    `id_suffix` is appended only to the pytest node id, not to `name` (which
    flows into `test_name` for BenchStore). Use it to keep ids unique when a
    driver reuses an existing suffix in a separate `cases()` call — e.g. a
    driver added later that shares the `""` suffix with drivers already in
    another `suffixes` dict would otherwise silently collide on id with them.
    """
    unique_suffixes = list(dict.fromkeys(s for ss in suffixes.values() for s in ss))
    type_keys = types if types is not None else (None,)
    params = []
    for row_count, label in sizes:
        for type_key in type_keys:
            for suffix in unique_suffixes:
                v = _variant(suffix)
                name = (
                    f"{type_key}_{label}{infix}{suffix}"
                    if type_key is not None
                    else f"{label}{infix}{suffix}"
                )
                values = (
                    (row_count, type_key, name, v.fetch_mode, v.bind_mode)
                    if type_key is not None
                    else (row_count, name, v.fetch_mode, v.bind_mode)
                )
                params.append(
                    pytest.param(
                        *values,
                        id=f"{name}{id_suffix}",
                        marks=pytest.mark.supported_drivers(*_drivers(suffix, suffixes)),
                    )
                )
    return params


def _variant(suffix: str) -> Variant:
    for variants in VARIANTS.values():
        for v in variants:
            if v.suffix == suffix:
                return v
    raise KeyError(f"No variant with suffix {suffix!r}")


def _drivers(suffix: str, suffixes) -> tuple[str, ...]:
    return tuple(driver for driver, ss in suffixes.items() if suffix in ss)
