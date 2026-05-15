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

    application: str | None = None
    """Application name override."""

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
            "application",
            "numpy",
            "arrow_number_to_decimal",
            "paramstyle",
            "autocommit",
            "connections_file_path",
            "auto_cleanup",
        }
    )
    """Fields handled only in Python, not forwarded to Rust."""

    _LEGACY_REWRITES: ClassVar[dict[str, str]] = {
        "private_key_file_pwd": "private_key_password",
        "client_request_mfa_token": "client_store_temporary_credential",
    }
    """Legacy parameter names -> canonical replacements."""

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
        """
        for old_name, new_name in cls._LEGACY_REWRITES.items():
            if old_name in kwargs:
                value = kwargs.pop(old_name)
                if new_name not in kwargs:
                    kwargs[new_name] = value

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
        * ``application`` - validates against ``_APPLICATION_RE``, defaults to
          ``_APPLICATION_NAME``, and injects ``client_app_id`` into
          ``_extra`` for the Rust core.
        * ``autocommit`` - type-checked (must be ``bool``), then merged into
          ``session_parameters["AUTOCOMMIT"]``.
        """
        if config is not None and kwargs:
            raise ProgrammingError(
                "Cannot pass both a ConnectionConfig object and keyword arguments. Use one or the other."
            )

        if config is None:
            if connection_name is not None:
                kwargs["connection_name"] = connection_name
            if connections_file_path is not None:
                kwargs["connections_file_path"] = connections_file_path
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

        application = config.application
        if application is None or (isinstance(application, str) and not application):
            config.application = cls._APPLICATION_NAME
        elif isinstance(application, str):
            if not cls._APPLICATION_RE.match(application):
                raise ProgrammingError(f"Invalid application name: {application!r}")
        else:
            raise ProgrammingError(f"Invalid application parameter (must be a non-empty string): {application!r}")
        config._extra["client_app_id"] = config.application

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
