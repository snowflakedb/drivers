"""Behavioural mixin for the generated :class:`ConnectionConfig`.

The concrete :class:`~snowflake.connector.connection_config.ConnectionConfig`
dataclass is auto-generated from the Rust ``PARAM_DEFS`` registry and contains
*only* information derivable from that source (field definitions plus the
alias / name / sensitive-param tables).

Everything else — Python-only fields, the ``_extra`` bag, legacy rewrites,
application-name validation and the ``to_options`` / ``to_proto_options``
pipeline — lives here, in a hand-written mixin that the generator does not
touch.
"""

from __future__ import annotations

import re
import warnings

from collections.abc import Callable, Iterable
from dataclasses import dataclass, field, fields
from typing import Any, ClassVar, TypeVar

from snowflake.connector._internal._private_key_helper import normalize_private_key
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ConfigSetting
from snowflake.connector.errors import ProgrammingError


_Self = TypeVar("_Self", bound="ConnectionConfigMixin")


OptionsModifier = Callable[[dict[str, Any]], dict[str, Any]]
"""Callable applied to the options dict produced by ``ConnectionConfig.to_options``.

A modifier may mutate the dict in place and/or return a new dict.  The return
value is fed into the next modifier in the chain.  See
``snowflake.connector._internal.logout_config_mapping.logout_config_options_modifier``
for an example modifier that re-applies legacy ``LogoutConfig`` semantics.
"""


@dataclass
class ConnectionConfigMixin:
    """Python-only fields and behavioural helpers for :class:`ConnectionConfig`.

    Fields defined here are never derived from ``PARAM_DEFS`` — they're wrapper-
    specific concerns (Python runtime flags, the ``_extra`` passthrough bag,
    and so on).  Class-level constants and methods implement the algorithmic
    pieces (alias resolution, application validation, protobuf conversion)
    that would otherwise need to be regenerated on every ``PARAM_DEFS`` change.

    The concrete generated :class:`ConnectionConfig` adds the PARAM_DEFS-derived
    fields and the static lookup tables (``_ALIAS_MAP``, ``_PYTHON_TO_RUST_NAME``,
    ``_SENSITIVE_PARAMS``) on top of this mixin.
    """

    # -- Python-only fields (not in PARAM_DEFS) --------------------------------
    session_parameters: dict[str, Any] | None = None
    """Session parameters to set at connection time."""

    numpy: bool | None = None
    """Use numpy for result set processing."""

    arrow_number_to_decimal: bool | None = None
    """Convert Arrow NUMBER columns to Python Decimal."""

    paramstyle: str | None = None
    """PEP 249 parameter binding style."""

    autocommit: bool | None = None
    """Enable/disable autocommit at connection time."""

    connections_file_path: str | None = None
    """Path to a TOML connections configuration file.

    Accepted by :class:`Connection` for forward-compatibility with
    snowflake-connector-python: callers can pass it today without breakage.
    Loading the file and merging the named ``connection_name`` profile into
    this config is **not yet wired up** in the universal driver; until that
    lands, the value is stored on the dataclass but does not influence
    connection behaviour.  TODO: integrate with ``ConfigManager``."""

    auto_cleanup: bool | None = None
    """Whether the connection should auto-close at interpreter shutdown via an
    ``atexit`` handler.  Python-only flag preserved for backward compatibility
    with snowflake-connector-python; never forwarded to the Rust core.  When
    ``None`` the wrapper treats it as ``True`` (legacy default)."""

    _extra: dict[str, Any] = field(default_factory=dict, repr=False)
    """Unknown kwargs forwarded to the Rust core for validation.

    Only values of the protobuf-supported scalar types (``bool``, ``int``,
    ``str``, ``float``, ``bytes``) survive the trip through
    :meth:`to_proto_options`; any other value type raises
    :class:`~snowflake.connector.errors.ProgrammingError` so misconfigurations
    fail loudly instead of being silently dropped.
    """

    # -- Hand-written class constants -----------------------------------------
    _PYTHON_ONLY: ClassVar[frozenset[str]] = frozenset(
        {
            "session_parameters",
            "numpy",
            "arrow_number_to_decimal",
            "paramstyle",
            "autocommit",
            "connections_file_path",
            "auto_cleanup",
        }
    )
    """Fields handled only in Python, not forwarded to Rust."""

    _INTERNAL_PARAMS: ClassVar[frozenset[str]] = frozenset(
        {
            "client_app_id",
            "client_app_version",
        }
    )
    """Driver-identity parameters that end users must not override.

    Mirrors the old connector's treatment of ``internal_application_name`` /
    ``internal_application_version``: these identify the driver family on the
    wire (CLIENT_APP_ID / CLIENT_APP_VERSION) and are owned by the wrapper, not
    the caller. Setting them via user-facing kwargs raises ``ProgrammingError``;
    the wrapper itself assigns them through ``ConnectionConfig`` attribute
    access in ``from_connection_args``."""

    _LEGACY_REWRITES: ClassVar[dict[str, str]] = {}
    """Legacy parameter names -> canonical replacements (silent)."""

    _DEPRECATED_REWRITES: ClassVar[dict[str, str]] = {
        "client_fetch_threads": "client_prefetch_threads",
        "client_request_mfa_token": "client_store_temporary_credential",
        "private_key_file_pwd": "private_key_password",
    }
    """Deprecated parameter names -> canonical replacements.

    Each hit emits a ``DeprecationWarning``.  Unlike ``_LEGACY_REWRITES`` these
    names are not silently supported forever — they exist so users migrating
    from ``snowflake-connector-python`` get a pointer to the new name.
    """

    _UNSUPPORTED_PARAMS: ClassVar[dict[str, str]] = {
        "client_fetch_use_mp": (
            "not supported; universal driver uses a thread pool for chunk fetch (see BehaviorDifferences)"
        ),
    }
    """Legacy kwargs that are accepted for source compatibility but have no effect."""

    _APPLICATION_NAME: ClassVar[str] = "PythonConnector"
    """Default application name."""

    _APPLICATION_RE: ClassVar[re.Pattern[str]] = re.compile(r"^[\w\d_]+")
    """Regex for validating application names."""

    # -- Generated-class hooks (populated by the concrete subclass) -----------
    # These ClassVars are re-declared on the generated ``ConnectionConfig`` so
    # that :meth:`from_kwargs` / :meth:`to_options` / :meth:`redacted_options`
    # can look them up via ``cls`` / ``self`` without the mixin having to know
    # the PARAM_DEFS contents.  Empty defaults keep the mixin usable in
    # isolation (tests, mypy) when no subclass is involved.
    _ALIAS_MAP: ClassVar[dict[str, str]] = {}
    """Lowercased alias -> Python field name. Populated by the generated subclass."""

    _PYTHON_TO_RUST_NAME: ClassVar[dict[str, str]] = {}
    """Python field name -> Rust canonical name. Populated by the generated subclass."""

    _SENSITIVE_PARAMS: ClassVar[frozenset[str]] = frozenset()
    """Fields that contain secrets. Populated by the generated subclass."""

    # -- Derived accessors ----------------------------------------------------
    @classmethod
    def _all_field_names(cls) -> frozenset[str]:
        """Names of every dataclass field (Python-only + PARAM_DEFS), sans the private ``_extra`` bag."""
        return frozenset(f.name for f in fields(cls) if not f.name.startswith("_"))

    # -- Factory methods ------------------------------------------------------
    @classmethod
    def from_kwargs(cls: type[_Self], **kwargs: Any) -> _Self:
        """Create a ConnectionConfig from keyword arguments.

        Resolves case-insensitive aliases, applies legacy parameter name
        rewrites, and collects unknown keys into ``_extra``.

        Raises ``ProgrammingError`` if a caller passes any of
        ``cls._INTERNAL_PARAMS`` (``client_app_id`` / ``client_app_version``).
        These identify the driver family on the wire and are wrapper-owned;
        end users must use ``application`` to label their app instead.
        """
        # Apply legacy rewrites first (silent — canonical replacements).
        for old_name, new_name in cls._LEGACY_REWRITES.items():
            if old_name in kwargs:
                value = kwargs.pop(old_name)
                if new_name not in kwargs:
                    kwargs[new_name] = value

        # Apply deprecated rewrites with a ``DeprecationWarning`` so callers
        # migrating from snowflake-connector-python see a pointer to the new
        # name.  ``stacklevel=3`` surfaces the caller of ``Connection(...)`` /
        # ``ConnectionConfig.from_kwargs(...)`` rather than this file.
        for old_name, new_name in cls._DEPRECATED_REWRITES.items():
            if old_name in kwargs:
                value = kwargs.pop(old_name)
                warnings.warn(
                    f"{old_name!r} is deprecated; use {new_name!r} instead.",
                    DeprecationWarning,
                    stacklevel=3,
                )
                if new_name not in kwargs:
                    kwargs[new_name] = value

        # Drop unsupported legacy kwargs with a warning so the caller knows
        # they had no effect instead of silently forwarding them to Rust.
        for key in list(kwargs):
            if key.lower() in cls._UNSUPPORTED_PARAMS:
                reason = cls._UNSUPPORTED_PARAMS[key.lower()]
                warnings.warn(
                    f"{key!r} has no effect in the universal driver: {reason}.",
                    DeprecationWarning,
                    stacklevel=3,
                )
                kwargs.pop(key)

        for key in kwargs:
            if key.lower() in cls._INTERNAL_PARAMS:
                raise ProgrammingError(
                    f"{key!r} is a wrapper-internal parameter and cannot be set "
                    f"by the caller. Use ``application`` to label your application."
                )

        known_fields = cls._all_field_names()
        resolved: dict[str, Any] = {}
        extra: dict[str, Any] = {}

        for key, value in kwargs.items():
            lower_key = key.lower()
            if lower_key in known_fields:
                resolved[lower_key] = value
            elif lower_key in cls._ALIAS_MAP:
                resolved[cls._ALIAS_MAP[lower_key]] = value
            else:
                extra[key] = value

        config = cls(**resolved)
        config._extra = extra
        return config

    @classmethod
    def from_connection_args(
        cls: type[_Self],
        connection_name: str | None = None,
        connections_file_path: str | None = None,
        config: _Self | None = None,
        **kwargs: Any,
    ) -> _Self:
        """Build a ConnectionConfig from Connection constructor arguments.

        Merges ``connection_name``, ``connections_file_path``, an optional
        pre-built ``config``, and any remaining ``**kwargs`` into a single
        :class:`ConnectionConfig`.  Raises ``ProgrammingError`` when both *config*
        and *kwargs* are supplied.

        Performs all value normalisation and validation so that the returned
        config is ready to be consumed by ``to_options()`` without further
        processing:

        * ``private_key`` - normalises RSAPrivateKey / bytes / str via
          :func:`normalize_private_key`.
        * ``application`` - validates against ``_APPLICATION_RE`` and defaults
          to ``_APPLICATION_NAME``. The value is forwarded to the Rust core as
          ``application`` and lands in CLIENT_ENVIRONMENT.APPLICATION on the
          wire. ``client_app_id`` (CLIENT_APP_ID) is the driver name by default
          so server-side feature gating tied to the client type keeps working;
          tools that re-host the driver (SnowSQL, Snow CLI) can override it via
          ``internal_application_name``.
        * ``internal_application_name`` / ``internal_application_version`` -
          popped from kwargs and mapped to ``client_app_id`` (CLIENT_APP_ID)
          and ``client_app_version`` (CLIENT_APP_VERSION) respectively. When
          omitted, ``client_app_id`` defaults to ``_APPLICATION_NAME`` and
          ``client_app_version`` is left unset so the Rust core falls back to
          its compiled-in default.
        * ``autocommit`` - type-checked (must be ``bool``), then merged into
          ``session_parameters["AUTOCOMMIT"]``.
        """
        if config is not None and kwargs:
            raise ProgrammingError(
                "Cannot pass both a ConnectionConfig object and keyword arguments. Use one or the other."
            )

        internal_app_name: Any = None
        internal_app_version: Any = None
        if config is None:
            if connection_name is not None:
                kwargs["connection_name"] = connection_name
            if connections_file_path is not None:
                kwargs["connections_file_path"] = connections_file_path
            # Pop the ``internal_application_*`` overrides before
            # ``from_kwargs`` runs: they are wrapper-internal levers that
            # ultimately populate ``client_app_id`` / ``client_app_version``,
            # which ``_INTERNAL_PARAMS`` forbids end users from setting
            # directly.
            internal_app_name = kwargs.pop("internal_application_name", None)
            internal_app_version = kwargs.pop("internal_application_version", None)
            config = cls.from_kwargs(**kwargs)
        else:
            if connection_name is not None:
                config.connection_name = connection_name  # type: ignore[attr-defined]
            if connections_file_path is not None:
                config.connections_file_path = connections_file_path

        # ``private_key`` is defined on the generated subclass (a PARAM_DEFS field);
        # mypy can't see it through the TypeVar, so the attribute access is suppressed.
        if config.private_key is not None:  # type: ignore[attr-defined]
            config.private_key = normalize_private_key(config.private_key)  # type: ignore[attr-defined]

        application = config.application  # type: ignore[attr-defined]
        if application is None or (isinstance(application, str) and not application):
            config.application = cls._APPLICATION_NAME  # type: ignore[attr-defined]
        elif isinstance(application, str):
            if not cls._APPLICATION_RE.match(application):
                raise ProgrammingError(f"Invalid application name: {application!r}")
        else:
            raise ProgrammingError(f"Invalid application parameter (must be a non-empty string): {application!r}")
        # CLIENT_APP_ID is the driver identity. Defaults to the driver name,
        # but can be overridden via ``internal_application_name`` (used e.g.
        # by SnowSQL / Snow CLI to identify themselves to the server).
        # CLIENT_ENVIRONMENT.APPLICATION (server-side ``application``) carries
        # the user-facing application name; the two values are independent.
        if isinstance(internal_app_name, str) and internal_app_name:
            config.client_app_id = internal_app_name  # type: ignore[attr-defined]
        else:
            config.client_app_id = cls._APPLICATION_NAME  # type: ignore[attr-defined]
        # CLIENT_APP_VERSION is only sent when the caller explicitly overrides
        # it via ``internal_application_version`` — otherwise the Rust core
        # falls back to its compiled-in default.
        if isinstance(internal_app_version, str) and internal_app_version:
            config.client_app_version = internal_app_version  # type: ignore[attr-defined]

        if config.autocommit is not None:
            if not isinstance(config.autocommit, bool):
                raise ProgrammingError(f"Invalid autocommit parameter: {config.autocommit!r}")
            if config.session_parameters is None:
                config.session_parameters = {}
            config.session_parameters["AUTOCOMMIT"] = str(config.autocommit).lower()

        return config

    # -- Serialisation --------------------------------------------------------
    def to_options(
        self,
        options_modifiers: Iterable[OptionsModifier] | None = None,
    ) -> dict[str, Any]:
        """Return non-None parameters for the Rust core.

        Excludes Python-only fields and maps Python field names back to
        Rust canonical names where they differ.  Includes ``_extra`` items.

        Args:
            options_modifiers: Optional iterable of callables applied to the
                options dict, in order, after it is built.  Each modifier
                receives the current dict and must return a dict (it may
                mutate the input in place).  The return value is passed to
                the next modifier.  Used to re-apply wrapper-specific
                legacy behaviour (e.g. ``LogoutConfig`` defaults & remap).
        """
        options: dict[str, Any] = {}

        for field_name in self._all_field_names():
            if field_name in self._PYTHON_ONLY:
                continue
            value = getattr(self, field_name, None)
            if value is None:
                continue
            rust_name = self._PYTHON_TO_RUST_NAME.get(field_name, field_name)
            options[rust_name] = value

        options.update(self._extra)

        if options_modifiers:
            for modifier in options_modifiers:
                options = modifier(options)
        return options

    def redacted_options(
        self,
        options_modifiers: Iterable[OptionsModifier] | None = None,
    ) -> dict[str, Any]:
        """Like ``to_options()`` but with sensitive values replaced by ``'***'``."""
        return {k: ("***" if k in self._SENSITIVE_PARAMS else v) for k, v in self.to_options(options_modifiers).items()}

    def to_proto_options(
        self,
        options_modifiers: Iterable[OptionsModifier] | None = None,
    ) -> dict[str, ConfigSetting]:
        """Convert ``to_options()`` into protobuf ``ConfigSetting`` messages.

        Raises:
            ProgrammingError: if any option value is not one of the protobuf-
                supported scalar types (``bool``, ``int``, ``str``, ``float``,
                ``bytes``).  This commonly indicates that an unsupported value
                slipped into ``_extra`` (e.g. a list, dict, or custom object);
                surface it instead of silently dropping the entry.
        """
        proto: dict[str, ConfigSetting] = {}
        for key, value in self.to_options(options_modifiers).items():
            # ``bool`` is a subclass of ``int``, so the bool branch has to come first.
            if isinstance(value, bool):
                proto[key] = ConfigSetting(bool_value=value)
            elif isinstance(value, int):
                proto[key] = ConfigSetting(int_value=value)
            elif isinstance(value, str):
                proto[key] = ConfigSetting(string_value=value)
            elif isinstance(value, float):
                proto[key] = ConfigSetting(double_value=value)
            elif isinstance(value, bytes):
                proto[key] = ConfigSetting(bytes_value=value)
            else:
                raise ProgrammingError(
                    f"Unsupported connection option type for {key!r}: "
                    f"{type(value).__name__}. Expected one of bool, int, "
                    f"str, float, or bytes."
                )
        return proto
