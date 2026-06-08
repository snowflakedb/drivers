"""Tests for ``@backward_compatibility`` + PEP 562 ``__getattr__`` warnings.

These tests cover both:

* the generic :func:`install_backward_compatibility_getattr` helper, exercised
  against a synthetic module, so we can assert behavior independent of which
  concrete names happen to live in ``snowflake.connector.errors`` today; and
* the concrete wiring in ``snowflake.connector.errors``, to make sure importing
  a real backward-compat exception emits the warning exactly once.

The autouse ``_reset_backward_compat_dedup_set`` fixture below snapshots and
restores the process-wide dedup set around every test in this module so
order-dependent false negatives cannot slip in.
"""

from __future__ import annotations

import importlib
import pkgutil
import sys
import types
import warnings

import pytest

from snowflake.connector._internal.backward_compatibility import (
    _BACKWARD_COMPAT_WARNED,
    _MARKED_BACKWARD_COMPAT,
    _wrap_fn_with_warning,
    install_backward_compatibility_getattr,
)
from snowflake.connector._internal.decorators import backward_compatibility


@pytest.fixture(autouse=True)
def _reset_backward_compat_dedup_set():
    """Snapshot and restore ``_BACKWARD_COMPAT_WARNED`` around each test.

    The set enforces once-per-process dedup for backward-compat warnings;
    without this fixture, the first test to observe a given warning would
    permanently mark that ``(module, name)`` slot as warned for the rest of
    the pytest session, silently hiding the warning from any later test
    that expects to observe it.

    Scoped to this module (rather than living in a shared ``conftest.py``)
    because no other unit test file touches this internal state.
    """
    snapshot = set(_BACKWARD_COMPAT_WARNED)
    _BACKWARD_COMPAT_WARNED.clear()
    try:
        yield
    finally:
        _BACKWARD_COMPAT_WARNED.clear()
        _BACKWARD_COMPAT_WARNED.update(snapshot)


def _make_synthetic_module(name: str) -> types.ModuleType:
    module = types.ModuleType(name)
    sys.modules[name] = module
    return module


class TestGenericHelper:
    def test_warns_once_on_first_access_and_returns_class(self):
        mod = _make_synthetic_module("snowflake.connector._test_bc_mod_a")

        @backward_compatibility
        class LegacyThing:
            pass

        LegacyThing.__module__ = mod.__name__
        mod.LegacyThing = LegacyThing

        install_backward_compatibility_getattr(mod.__name__)

        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            first = mod.LegacyThing
            second = mod.LegacyThing
            third = mod.LegacyThing

        assert first is LegacyThing
        assert second is LegacyThing
        assert third is LegacyThing

        bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert len(bc_warnings) == 1
        assert "LegacyThing" in str(bc_warnings[0].message)
        assert mod.__name__ in str(bc_warnings[0].message)

    def test_name_is_removed_from_module_globals(self):
        mod = _make_synthetic_module("snowflake.connector._test_bc_mod_b")

        @backward_compatibility
        class Shim:
            pass

        Shim.__module__ = mod.__name__
        mod.Shim = Shim

        assert "Shim" in vars(mod)
        install_backward_compatibility_getattr(mod.__name__)
        assert "Shim" not in vars(mod)
        # Still resolvable via attribute lookup (triggers __getattr__).
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            assert mod.Shim is Shim

    def test_non_decorated_names_untouched(self):
        mod = _make_synthetic_module("snowflake.connector._test_bc_mod_c")

        class Kept:
            pass

        Kept.__module__ = mod.__name__
        mod.Kept = Kept

        @backward_compatibility
        class Moved:
            pass

        Moved.__module__ = mod.__name__
        mod.Moved = Moved

        install_backward_compatibility_getattr(mod.__name__)

        assert "Kept" in vars(mod)
        assert "Moved" not in vars(mod)

        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            _ = mod.Kept  # no warning
            _ = mod.Moved  # one warning

        bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert len(bc_warnings) == 1

    def test_unknown_attribute_still_raises_attribute_error(self):
        mod = _make_synthetic_module("snowflake.connector._test_bc_mod_d")

        @backward_compatibility
        class X:
            pass

        X.__module__ = mod.__name__
        mod.X = X

        install_backward_compatibility_getattr(mod.__name__)

        with pytest.raises(AttributeError):
            mod.DoesNotExist  # noqa: B018 - intentional attribute access

    def test_descriptor_is_returned_unchanged(self):
        """The decorator must not mutate descriptors (e.g. ``property``) — an
        earlier implementation stamped a marker attribute on the target, which
        silently dropped ``property`` instances (they reject attribute
        assignment). The registry-based implementation tracks membership
        externally, so the decorator site gets the same object back."""
        p = property(lambda self: 1)
        assert backward_compatibility(p) is p
        # The unchanged property should still behave as a property.
        assert isinstance(backward_compatibility(p), property)

    def test_reexports_from_other_modules_not_stashed(self):
        """A class defined in another module and re-bound here should be left alone."""
        mod = _make_synthetic_module("snowflake.connector._test_bc_mod_e")

        @backward_compatibility
        class Foreign:
            pass

        # Simulate "defined elsewhere, re-exported here".
        Foreign.__module__ = "some.other.module"
        mod.Foreign = Foreign

        install_backward_compatibility_getattr(mod.__name__)

        # Still present as a direct global — no __getattr__ indirection.
        assert "Foreign" in vars(mod)


class TestErrorsModuleIntegration:
    """Integration tests for ``snowflake.connector.errors``.

    We deliberately do NOT ``sys.modules.pop("snowflake.connector.errors")``
    to force re-import: other modules hold references to classes like
    ``Error`` by identity (e.g. ``isinstance(exc, Error)`` checks), and a
    fresh re-import would shadow the canonical class object and break them.
    The autouse ``_reset_backward_compat_dedup_set`` fixture in
    ``conftest.py`` is enough to make each warning-emit test start from a
    clean slate — the PEP 562 ``__getattr__`` will re-fire on the next
    access of a stashed name.
    """

    def test_backward_compat_class_warns_once_on_import(self):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            from snowflake.connector.errors import BadGatewayError  # noqa: F401

            # A second import of the same name should NOT produce another warning.
            from snowflake.connector.errors import BadGatewayError as _again  # noqa: F401

        bc_warnings = [
            w for w in caught if issubclass(w.category, DeprecationWarning) and "BadGatewayError" in str(w.message)
        ]
        assert len(bc_warnings) == 1, [str(w.message) for w in caught]

    def test_pep249_classes_do_not_warn(self):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            from snowflake.connector.errors import (  # noqa: F401
                DatabaseError,
                Error,
                ProgrammingError,
            )

        bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert bc_warnings == []

    def test_backward_compat_class_still_usable_after_warning(self):
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            from snowflake.connector.errors import BadGatewayError, Error

        # Subclass of Error, raisable, carries errno/sqlstate like a normal Error.
        assert issubclass(BadGatewayError, Error)
        with pytest.raises(BadGatewayError) as excinfo:
            raise BadGatewayError("boom", errno=502)
        assert excinfo.value.errno == 502


class TestResultBatchModuleIntegration:
    """Mirror of :class:`TestErrorsModuleIntegration` for ``result_batch``:
    the module installs ``__getattr__`` so legacy imports of
    ``ArrowResultBatch`` and ``JSONResultBatch`` warn once per process but
    stay usable. Same re-import rationale as ``errors`` applies: ``ResultBatch``
    is pickled elsewhere, and re-importing the module would break identity.
    """

    @pytest.mark.parametrize("class_name", ["ArrowResultBatch", "JSONResultBatch"])
    def test_backward_compat_class_warns_once_on_import(self, class_name):
        import snowflake.connector.result_batch as result_batch_module

        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            first = getattr(result_batch_module, class_name)
            second = getattr(result_batch_module, class_name)  # second access: deduped
            assert first is second

        bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning) and class_name in str(w.message)]
        assert len(bc_warnings) == 1, [str(w.message) for w in caught]

    def test_result_batch_base_class_does_not_warn(self):
        """``ResultBatch`` itself is the active class; resolving it must be silent."""
        import snowflake.connector.result_batch as result_batch_module

        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            _ = result_batch_module.ResultBatch

        bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert bc_warnings == []

    def test_backward_compat_subclass_is_still_a_resultbatch(self):
        """After the warning fires, the stashed class must still be a true
        subclass of ``ResultBatch`` so ``isinstance`` checks in user code
        (the whole point of preserving these names) keep working."""
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            from snowflake.connector.result_batch import (
                ArrowResultBatch,
                JSONResultBatch,
                ResultBatch,
            )

        assert issubclass(ArrowResultBatch, ResultBatch)
        assert issubclass(JSONResultBatch, ResultBatch)


class TestCallTimeWarning:
    """``@backward_compatibility`` on a function should warn on first external
    call, stay silent for internal callers, and share the dedup slot with the
    module ``__getattr__`` path."""

    def test_top_level_function_warns_on_call(self):
        # IS_UNICODE is decorated with @backward_compatibility in compat.py.
        from snowflake.connector.compat import IS_UNICODE

        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            assert IS_UNICODE("g") is True
            # Second call: no additional warning (once-per-process dedup).
            assert IS_UNICODE("h") is True

        bc_warnings = [
            w for w in caught if issubclass(w.category, DeprecationWarning) and "IS_UNICODE" in str(w.message)
        ]
        assert len(bc_warnings) == 1, [str(w.message) for w in caught]

    def test_internal_caller_does_not_warn(self):
        """Calls originating from ``snowflake.connector.*`` must not consume
        the one-shot warning slot."""
        from snowflake.connector._internal.backward_compatibility import _BACKWARD_COMPAT_WARNED
        from snowflake.connector.compat import IS_UNICODE

        # Impersonate an internal caller by executing the call inside a module
        # whose __name__ starts with "snowflake.connector".
        ns: dict = {"IS_UNICODE": IS_UNICODE, "__name__": "snowflake.connector.fake_internal"}
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            exec("result = IS_UNICODE('x')", ns)

        assert ns["result"] is True
        bc_warnings = [
            w for w in caught if issubclass(w.category, DeprecationWarning) and "IS_UNICODE" in str(w.message)
        ]
        assert bc_warnings == [], [str(w.message) for w in caught]
        # The slot must still be free for a future external call.
        assert ("snowflake.connector.compat", "IS_UNICODE") not in _BACKWARD_COMPAT_WARNED

    def test_method_on_class_warns_for_external_callers_only(self):
        """Methods decorated with @backward_compatibility should warn only
        when called from outside the connector package."""

        # Build a decorated method on a class whose module looks like ours.
        from snowflake.connector._internal.decorators import backward_compatibility

        class _Holder:
            @backward_compatibility
            def legacy_method(self, x: int) -> int:
                return x + 1

        _Holder.__module__ = "snowflake.connector._test_bc_methods"
        _Holder.legacy_method.__module__ = "snowflake.connector._test_bc_methods"

        # Internal caller: pretend this test lives inside snowflake.connector.
        ns_internal: dict = {
            "h": _Holder(),
            "__name__": "snowflake.connector.some_internal_module",
        }
        ns_external: dict = {
            "h": _Holder(),
            "__name__": "customer.app.main",
        }

        with warnings.catch_warnings(record=True) as caught_internal:
            warnings.simplefilter("always")
            exec("r = h.legacy_method(1)", ns_internal)
        assert ns_internal["r"] == 2
        internal_warnings = [
            w
            for w in caught_internal
            if issubclass(w.category, DeprecationWarning) and "legacy_method" in str(w.message)
        ]
        assert internal_warnings == []

        with warnings.catch_warnings(record=True) as caught_external:
            warnings.simplefilter("always")
            exec("r = h.legacy_method(2)", ns_external)
            exec("r = h.legacy_method(3)", ns_external)  # second call: deduped
        assert ns_external["r"] == 4
        external_warnings = [
            w
            for w in caught_external
            if issubclass(w.category, DeprecationWarning) and "legacy_method" in str(w.message)
        ]
        assert len(external_warnings) == 1

    def test_module_access_and_function_call_share_dedup_slot(self):
        """White-box regression guard for the shared-dedup-set invariant.

        Registering a wrapped function under the same ``(module, name)`` key
        as a stashed class is a synthetic setup that does not occur in the
        production codebase — but both paths writing into the same
        ``_BACKWARD_COMPAT_WARNED`` set is exactly the invariant we rely on
        for once-per-symbol dedup, so it's worth pinning down: whichever
        path fires first must consume the slot for both.
        """
        mod_name = "snowflake.connector._test_bc_shared_paths"
        mod = _make_synthetic_module(mod_name)

        @backward_compatibility
        class SharedName:
            pass

        SharedName.__module__ = mod_name
        mod.SharedName = SharedName

        install_backward_compatibility_getattr(mod_name)

        # A wrapped function registered under the *same* (module, name) key
        # as the class stashed above, so both paths hit one dedup slot.
        def _impl(x: int) -> int:
            return x

        _impl.__module__ = mod_name
        _impl.__qualname__ = "SharedName"
        wrapped = _wrap_fn_with_warning(_impl)

        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            _ = mod.SharedName  # module __getattr__ path: emits
            assert wrapped(1) == 1  # call-wrapper path: deduped by shared set
            _ = mod.SharedName  # second access: deduped

        bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert len(bc_warnings) == 1, [str(w.message) for w in caught]

    def test_module_getattr_does_not_warn_for_internal_callers(self):
        """Module ``__getattr__`` accesses originating from inside
        ``snowflake.connector.*`` must not consume the one-shot warning slot,
        mirroring the call-wrapper's internal-caller filter."""
        from snowflake.connector._internal.backward_compatibility import _BACKWARD_COMPAT_WARNED

        mod_name = "snowflake.connector._test_bc_internal_getattr"
        mod = _make_synthetic_module(mod_name)

        @backward_compatibility
        class Legacy:
            pass

        Legacy.__module__ = mod_name
        mod.Legacy = Legacy

        install_backward_compatibility_getattr(mod_name)

        # Simulate an internal access by executing ``mod.Legacy`` inside a
        # namespace whose ``__name__`` starts with ``snowflake.connector``.
        ns: dict = {"mod": sys.modules[mod_name], "__name__": "snowflake.connector.fake_internal"}
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            exec("value = mod.Legacy", ns)

        assert ns["value"] is Legacy
        bc_warnings = [w for w in caught if issubclass(w.category, DeprecationWarning)]
        assert bc_warnings == [], [str(w.message) for w in caught]
        # Slot must still be free for a future external access.
        assert (mod_name, "Legacy") not in _BACKWARD_COMPAT_WARNED


class TestNoInternalImportsOfBackwardCompatNames:
    """Source-level guard: no ``snowflake.connector.*`` module may rebind a
    ``@backward_compatibility``-decorated class into its own globals.

    The ``__getattr__`` internal-caller filter means such an import won't
    surface a user-facing warning, but it still re-entangles driver internals
    with the legacy surface we're trying to retire. Flagging it at test time
    keeps the one-way "internals → legacy" dependency from creeping back in.
    """

    def test_no_internal_module_imports_a_stashed_backward_compat_name(self):
        import snowflake.connector

        # Discover every real driver submodule on disk. We deliberately scope
        # to what ``pkgutil.walk_packages`` returns (plus the root package)
        # so that synthetic ``snowflake.connector._test_bc_*`` modules
        # created by earlier tests in this file are never inspected.
        real_module_names: set[str] = {"snowflake.connector"}
        for info in pkgutil.walk_packages(snowflake.connector.__path__, prefix="snowflake.connector."):
            real_module_names.add(info.name)
            try:
                importlib.import_module(info.name)
            except ImportError:
                # Optional-dependency submodules are allowed to skip.
                continue

        offenders: list[str] = []
        for module_name in real_module_names:
            module = sys.modules.get(module_name)
            if module is None:
                continue
            for attr_name, value in vars(module).items():
                if attr_name.startswith("_"):
                    continue
                try:
                    is_marked = value in _MARKED_BACKWARD_COMPAT
                except TypeError:
                    # Unhashable module-level objects (e.g. imported dicts)
                    # cannot be registry members by construction; skip.
                    continue
                if not is_marked:
                    continue
                defining_module = getattr(value, "__module__", None)
                if defining_module and defining_module != module_name:
                    offenders.append(f"{module_name}.{attr_name} (defined in {defining_module})")

        assert not offenders, (
            "Internal snowflake.connector modules must not re-bind "
            "@backward_compatibility-decorated names into their own globals "
            "(this re-couples driver internals to the legacy surface). "
            "Offenders:\n  " + "\n  ".join(offenders)
        )
