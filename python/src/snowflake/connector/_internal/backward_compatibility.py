"""Runtime machinery backing the ``@backward_compatibility`` decorator.

The public decorator itself lives in :mod:`.decorators` for discoverability;
everything that supports it — the call-time wrapper, the module-level
``__getattr__`` installer, the dedup state, and the caller-origin check —
lives here so ``decorators.py`` stays a thin file of marker/annotation
decorators.

Three collaborators form the public surface of this module:

* :func:`apply_backward_compatibility` — used by the decorator in
  :mod:`.decorators`; never called directly by user code.
* :func:`install_backward_compatibility_getattr` — called at the bottom of
  modules that export ``@backward_compatibility``-decorated classes, to wire
  up the PEP 562 ``__getattr__`` that warns on first attribute access.
* :data:`_BACKWARD_COMPAT_WARNED` — process-wide dedup set; exported for
  tests only.
"""

from __future__ import annotations

import functools
import inspect
import sys
import warnings

from collections.abc import Callable
from typing import Any, TypeVar


F = TypeVar("F", bound=Callable)
T = TypeVar("T")


# Callers whose module name starts with any of these prefixes are considered
# internal and will NOT trigger a backward-compatibility warning. This keeps
# internal uses (e.g. ``self.populate_data(...)`` called from elsewhere in the
# driver) silent so the dedup slot remains available for the first external use.
_INTERNAL_CALLER_PREFIX = "snowflake.connector"

# (module, qualname) pairs that have already emitted a deprecation warning.
# Used to enforce once-per-process dedup across both the ``__getattr__`` and
# call-wrapper paths.
_BACKWARD_COMPAT_WARNED: set[tuple[str, str]] = set()

# Registry of objects processed by ``@backward_compatibility``. We track
# membership here rather than stamping the decorated object with a marker
# attribute: (a) descriptors such as ``property`` reject arbitrary attribute
# assignment, so the marker approach silently failed for them, and (b) the
# decorator stays non-invasive — the decorated object is never mutated.
# Entries are held by identity under the default hash (classes, functions,
# and stdlib descriptors are all hashable that way).
_MARKED_BACKWARD_COMPAT: set[Any] = set()

_DEPRECATION_WARNING_MSG_SUFFIX = (
    "is retained only for backward compatibility with snowflake-connector-python "
    "and is not used by the Universal Driver; "
    "it may be removed in a future release."
)


def apply_backward_compatibility(obj: T) -> T:
    """Implement the ``@backward_compatibility`` decorator.

    Plain functions are wrapped so the first *external* call emits a warning.
    Classes and descriptors (property, staticmethod, classmethod, etc.) are
    passed through untouched; for classes the module-level ``__getattr__``
    installed by :func:`install_backward_compatibility_getattr` turns
    registry membership into a warn-on-first-access behavior.

    The returned object is recorded in :data:`_MARKED_BACKWARD_COMPAT` so
    :func:`install_backward_compatibility_getattr` can later identify which
    module globals to stash, and so repeated ``@backward_compatibility``
    decoration is a no-op.
    """
    if _is_marked_backward_compat(obj):
        return obj  # idempotent re-decoration

    result = _wrap_fn_with_warning(obj) if inspect.isfunction(obj) else obj
    _mark_backward_compat(result)
    return result


def install_backward_compatibility_getattr(module_name: str) -> None:
    """Install a module-level ``__getattr__`` that warns on first access of
    each ``@backward_compatibility``-decorated top-level **class**.

    Must be called at the *bottom* of the target module, after all decorated
    classes have been defined. The tagged classes are removed from the
    module's globals and stashed privately so PEP 562 ``__getattr__`` is
    invoked on access — which is what lets us distinguish "someone imported
    this name" from mere module import.

    Top-level functions decorated with ``@backward_compatibility`` are NOT
    moved here; they already emit a warning on first external call via the
    call-time wrapper installed by :func:`apply_backward_compatibility`.
    """
    module = sys.modules[module_name]
    stash: dict[str, Any] = {}

    for name, value in list(vars(module).items()):
        if name.startswith("_"):
            continue
        # module-level __getattr__ only intercepts classes; functions self-warn
        # via the call wrapper installed by ``apply_backward_compatibility``.
        if not inspect.isclass(value):
            continue
        if not _is_marked_backward_compat(value):
            continue
        # only stash classes that were defined in this module, not re-exports.
        if getattr(value, "__module__", None) != module_name:
            continue
        stash[name] = value
        delattr(module, name)

    if not stash:
        return

    existing_getattr: Callable[[str], Any] | None = module.__dict__.get("__getattr__")
    module.__getattr__ = _module_getattr_with_warning(  # type: ignore[method-assign]
        module_name, stash, existing_getattr
    )


def _mark_backward_compat(obj: Any) -> None:
    _MARKED_BACKWARD_COMPAT.add(obj)


def _is_marked_backward_compat(obj: Any) -> bool:
    return obj in _MARKED_BACKWARD_COMPAT


def _wrap_fn_with_warning(obj: F) -> F:
    module = getattr(obj, "__module__", None) or ""
    # Prefer __qualname__ so method warnings read "Class.method" rather than just "method".
    name = getattr(obj, "__qualname__", None) or getattr(obj, "__name__", None) or ""

    @functools.wraps(obj)
    def wrapper(*args: Any, **kwargs: Any) -> Any:
        if _is_caller_external():
            _emit_backward_compatibility_warning(module, name)
        return obj(*args, **kwargs)

    return wrapper  # type: ignore[return-value]


def _is_caller_external() -> bool:
    """Return ``True`` when the frame that invoked our direct caller is NOT
    part of the ``snowflake.connector`` package tree.

    Used from both the call-time wrapper (:func:`_wrap_fn_with_warning`) and
    the PEP 562 ``__getattr__`` installed by
    :func:`install_backward_compatibility_getattr` to suppress warnings for
    accesses originating inside ``snowflake.connector.*`` — driver-internal
    use must never consume the once-per-process dedup slot before a customer
    does. Both call sites sit exactly one Python frame below the caller we
    care about, which is why the skip count below is fixed at 2.
    """
    try:
        frame = sys._getframe(2)  # skip: this helper + our direct caller
    except ValueError:
        return True
    caller_module = frame.f_globals.get("__name__") or ""
    return not caller_module.startswith(_INTERNAL_CALLER_PREFIX)


def _module_getattr_with_warning(
    module_name: str,
    stash: dict[str, Any],
    existing_getattr: Callable[[str], Any] | None,
) -> Callable[[str], Any]:
    """Build the PEP 562 ``__getattr__`` that emits a warning on first access
    of a stashed backward-compat name and delegates unknown names to any
    pre-existing module ``__getattr__``.

    Internal accesses (from within ``snowflake.connector.*``) bypass the
    warning via :func:`_is_caller_external` — the value is still returned so
    nothing breaks if a driver module does end up importing one of these
    names, but the one-shot dedup slot stays available for the first real
    user. A static test enforces that no such internal access exists today.
    """

    def __getattr__(name: str) -> Any:
        if name in stash:
            if _is_caller_external():
                _emit_backward_compatibility_warning(module_name, name)
            return stash[name]
        if existing_getattr is not None:
            return existing_getattr(name)
        raise AttributeError(f"module {module_name!r} has no attribute {name!r}")

    return __getattr__


def _emit_backward_compatibility_warning(module: str, name: str) -> None:
    """Emit a ``DeprecationWarning`` the first time ``module.name`` is used.

    Deduplication is done against an explicit set so the warning is emitted at
    most once per process per ``(module, name)`` pair regardless of the user's
    ``warnings`` filter configuration, and regardless of whether the first use
    was an import (module ``__getattr__``) or a call (wrapped callable).

    Concurrency: the check-then-add is not locked. Two threads racing on the
    same key may each see it as absent and both emit the warning once — an
    acceptable trade-off vs. taking a lock on every call.

    Stack-level invariant: this function must be called from exactly one
    Python frame below the user — i.e. from the wrapped-callable wrapper or
    from the module ``__getattr__``, and nowhere else. Given that, the two
    call chains are ``user -> wrapper -> _emit -> warn`` and
    ``user -> __getattr__ -> _emit -> warn``, so ``stacklevel=3`` puts the
    reported source at the user's frame in both cases. If you add a third
    call site, keep this invariant or this helper's stacklevel must grow a
    parameter.
    """
    key = (module, name)
    if key in _BACKWARD_COMPAT_WARNED:
        return
    _BACKWARD_COMPAT_WARNED.add(key)
    warnings.warn(
        f"'{module}.{name}' {_DEPRECATION_WARNING_MSG_SUFFIX}",
        DeprecationWarning,
        stacklevel=3,
    )
