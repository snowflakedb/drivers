//! Generates the Python `ConnectionConfig` dataclass from the canonical
//! `PARAM_DEFS` registry.  The output is a ready-to-use Python source file
//! printed to stdout, consumed by the `generate-connection-config` pre-commit
//! hook whenever `param_registry.rs` changes.

use sf_core::config::param_registry::{ParamDef, ParamScope, Required, ValueType, registry};
use sf_core::config::settings::Setting;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Map a Rust `ValueType` to the corresponding Python type annotation.
fn py_type(vt: ValueType) -> &'static str {
    match vt {
        ValueType::String => "str",
        ValueType::Int => "int",
        ValueType::Double => "float",
        ValueType::Bytes => "bytes",
        ValueType::Bool => "bool",
    }
}

/// Convert a canonical name to a Python-friendly snake_case identifier.
/// Most canonical names are already snake_case. Supports two other forms:
///
/// * camelCase (e.g. `passcodeInPassword`) — a `_` is inserted before each
///   internal uppercase letter, then the whole string is lowercased.
/// * SCREAMING_SNAKE_CASE (e.g. `CLIENT_PREFETCH_THREADS`) — already has
///   underscores, so it's simply lowercased.
fn to_python_field(canonical: &str) -> String {
    // Names that already use underscores (snake_case or SCREAMING_SNAKE_CASE)
    // just need lowercasing. Splitting every uppercase letter would turn
    // `CLIENT_PREFETCH_THREADS` into `c_l_i_e_n_t__p_r_e_f_e_t_c_h__...`.
    if canonical.contains('_') {
        return canonical.to_ascii_lowercase();
    }

    let mut result = String::with_capacity(canonical.len() + 4);
    for (i, ch) in canonical.chars().enumerate() {
        if ch.is_ascii_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_ascii_lowercase());
    }
    result
}

/// Format a `Setting` value as a Python literal that is safe to paste into
/// generated source code.
///
/// String / byte values escape backslashes, single quotes, and any character
/// that would otherwise terminate the literal or render it ambiguous (control
/// chars, non-graphic bytes).  This matters because callers occasionally
/// embed the result back into the generated Python (e.g. `Default: ...` in
/// docstrings); a bare `\` or `'` would break the surrounding triple-quoted
/// literal otherwise.
fn setting_to_py_literal(s: &Setting) -> String {
    match s {
        Setting::String(v) => {
            let mut escaped = String::with_capacity(v.len() + 2);
            escaped.push('\'');
            for ch in v.chars() {
                match ch {
                    '\\' => escaped.push_str("\\\\"),
                    '\'' => escaped.push_str("\\'"),
                    '\n' => escaped.push_str("\\n"),
                    '\r' => escaped.push_str("\\r"),
                    '\t' => escaped.push_str("\\t"),
                    c if (c as u32) < 0x20 => {
                        escaped.push_str(&format!("\\x{:02x}", c as u32));
                    }
                    c => escaped.push(c),
                }
            }
            escaped.push('\'');
            escaped
        }
        Setting::Int(v) => v.to_string(),
        Setting::Double(v) => v.to_string(),
        Setting::Bool(v) => {
            if *v {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        Setting::Bytes(v) => {
            let mut out = String::from("b'");
            for &b in v.iter() {
                if b == b'\\' || b == b'\'' || !b.is_ascii_graphic() {
                    out.push_str(&format!("\\x{:02x}", b));
                } else {
                    out.push(b as char);
                }
            }
            out.push('\'');
            out
        }
    }
}

/// Format the Required enum as a human-readable string for docstrings.
fn required_to_str(r: &Required) -> &'static str {
    match r {
        Required::Always => "always",
        Required::WhenAuthMethod(_) => "conditional",
        Required::Never => "never",
    }
}

/// Section comment from the scope.
fn scope_section(scope: ParamScope) -> &'static str {
    match scope {
        ParamScope::Connection => "Connection",
        ParamScope::Session => "Session",
        ParamScope::Statement => "Statement",
    }
}

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

fn generate(params: &[ParamDef]) -> String {
    let mut out = String::with_capacity(16 * 1024);

    // PARAM_DEFS may interleave scopes / be in registration order.  Sort by
    // ``(scope, canonical_name)`` so the dataclass fields appear in a single
    // contiguous block per scope (no repeated section headers) and the
    // generated file is stable across rebuilds.
    let mut sorted_params: Vec<&ParamDef> = params.iter().collect();
    sorted_params.sort_by(|a, b| {
        (scope_section(a.scope), a.canonical_name).cmp(&(scope_section(b.scope), b.canonical_name))
    });

    // ── Header ─────────────────────────────────────────────────────────
    out.push_str(
        r#""""Auto-generated from PARAM_DEFS in sf_core. DO NOT EDIT.

Regenerated by the generate-connection-config pre-commit hook whenever
param_registry.rs changes.  Any manual changes will be silently overwritten.
"""
from __future__ import annotations

import re

from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from typing import Any, ClassVar

from snowflake.connector._internal._private_key_helper import normalize_private_key
from snowflake.connector._internal.protobuf_gen.database_driver_v1_pb2 import ConfigSetting
from snowflake.connector.errors import ProgrammingError

OptionsModifier = Callable[[dict[str, Any]], dict[str, Any]]
"""Callable applied to the options dict produced by ``ConnectionConfig.to_options``.

A modifier may mutate the dict in place and/or return a new dict.  The return
value is fed into the next modifier in the chain.  See
``snowflake.connector._internal.logout_config_mapping.logout_config_options_modifier``
for an example modifier that re-applies legacy ``LogoutConfig`` semantics.
"""


"#,
    );

    // ── Collect metadata for class-level dicts ─────────────────────────
    // alias_map: lowercased alias -> python field name
    let mut alias_entries: Vec<(String, String)> = Vec::new();
    // python_to_rust: python field -> rust canonical (only when they differ)
    let mut py_to_rust: Vec<(String, String)> = Vec::new();
    // sensitive params
    let mut sensitive: Vec<String> = Vec::new();

    for p in sorted_params.iter().copied() {
        // Skip statement-scoped params; they belong on the cursor, not the connection.
        if p.scope == ParamScope::Statement {
            continue;
        }

        let py_field = to_python_field(p.canonical_name);

        // Canonical name itself as an alias (lowercased)
        let canonical_lower = p.canonical_name.to_ascii_lowercase();
        if canonical_lower != py_field {
            alias_entries.push((canonical_lower.clone(), py_field.clone()));
        }

        for alias in p.aliases {
            let alias_lower = alias.to_ascii_lowercase();
            // Skip aliases that already match the python field (identical name)
            // or the canonical's lowercase (already emitted above) to avoid
            // duplicate dict keys in the generated ``_ALIAS_MAP``.
            if alias_lower != py_field && alias_lower != canonical_lower {
                alias_entries.push((alias_lower, py_field.clone()));
            }
        }

        if py_field != p.canonical_name {
            py_to_rust.push((py_field.clone(), p.canonical_name.to_string()));
        }

        if p.sensitive {
            sensitive.push(py_field);
        }
    }

    alias_entries.sort();
    py_to_rust.sort();
    sensitive.sort();

    // ── Dataclass ──────────────────────────────────────────────────────
    out.push_str("@dataclass\nclass ConnectionConfig:\n");
    out.push_str("    \"\"\"Typed connection configuration derived from sf_core PARAM_DEFS.\n\n");
    out.push_str("    Fields use defaults from the Rust core where defined, ``None`` otherwise.\n");
    out.push_str("    \"\"\"\n\n");

    // Fields grouped by scope section
    let mut current_scope: Option<ParamScope> = None;

    for p in sorted_params.iter().copied() {
        // Skip statement-scoped params; they belong on the cursor, not the connection.
        if p.scope == ParamScope::Statement {
            continue;
        }

        if current_scope != Some(p.scope) {
            current_scope = Some(p.scope);
            out.push_str(&format!(
                "    # -- {} parameters {}\n",
                scope_section(p.scope),
                "-".repeat(50)
            ));
        }

        let py_field = to_python_field(p.canonical_name);

        // Build type annotation.  ``private_key`` is intentionally widened
        // because ``normalize_private_key()`` accepts cryptography
        // ``RSAPrivateKey`` instances in addition to ``str | bytes``; a
        // narrower hint would make mypy / IDEs reject perfectly valid
        // callers.  The runtime still validates / normalises before the
        // value is forwarded to the Rust core.
        let type_ann = if py_field == "private_key" {
            "Any | None".to_string()
        } else if let Some(extra) = p.additional_value_type {
            format!("{} | {} | None", py_type(p.value_type), py_type(extra))
        } else {
            format!("{} | None", py_type(p.value_type))
        };

        // Use actual default from core when available, otherwise None
        let default_literal = if let Some(ref default_fn) = p.default {
            setting_to_py_literal(&default_fn())
        } else {
            "None".to_string()
        };
        out.push_str(&format!(
            "    {}: {} = {}\n",
            py_field, type_ann, default_literal
        ));

        // Docstring with description and metadata
        let mut doc_parts = vec![p.description.to_string()];
        if let Some(ref default_fn) = p.default {
            let default_val = default_fn();
            doc_parts.push(format!("Default: {}", setting_to_py_literal(&default_val)));
        }
        let req_str = required_to_str(&p.required);
        if req_str != "never" {
            if let Required::WhenAuthMethod(method) = &p.required {
                doc_parts.push(format!("Required when authenticator={}", method));
            } else {
                doc_parts.push("Required".to_string());
            }
        }
        if let Some(dep) = p.deprecated_by {
            doc_parts.push(format!("Deprecated: use {} instead", to_python_field(dep)));
        }
        out.push_str(&format!("    \"\"\"{}\"\"\"\n\n", doc_parts.join(". ")));
    }

    // ── Python-only fields ─────────────────────────────────────────────
    out.push_str(
        r#"    # -- Python-only parameters (not in PARAM_DEFS) --------------------
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

    converter_class: Any | None = None
    """Custom ``SnowflakeConverter`` subclass (accepted for backward compatibility).

    In the legacy snowflake-connector-python driver this selected the Python
    class responsible for Snowflake-to-Python type conversion.  The universal
    driver performs conversion in its Rust / Arrow layer; the value is retained
    so ``connection.converter_class`` and ``connection.converter`` keep
    returning the expected type, but it does not influence conversion."""

"#,
    );

    // _extra field
    out.push_str("    _extra: dict[str, Any] = field(default_factory=dict, repr=False)\n");
    out.push_str(
        "    \"\"\"Unknown kwargs forwarded to the Rust core for validation.\n\n    Only values of the protobuf-supported scalar types (``bool``, ``int``,\n    ``str``, ``float``, ``bytes``) survive the trip through\n    :meth:`to_proto_options`; any other value type raises\n    :class:`~snowflake.connector.errors.ProgrammingError` so misconfigurations\n    fail loudly instead of being silently dropped.\n    \"\"\"\n\n",
    );

    // ── Class variables ────────────────────────────────────────────────
    // _ALIAS_MAP
    out.push_str("    _ALIAS_MAP: ClassVar[dict[str, str]] = {\n");
    for (alias, field_name) in &alias_entries {
        out.push_str(&format!("        \"{}\": \"{}\",\n", alias, field_name));
    }
    out.push_str("    }\n");
    out.push_str("    \"\"\"Lowercased alias -> Python field name.\"\"\"\n\n");

    // _PYTHON_TO_RUST_NAME
    out.push_str("    _PYTHON_TO_RUST_NAME: ClassVar[dict[str, str]] = {\n");
    for (py, rust) in &py_to_rust {
        out.push_str(&format!("        \"{}\": \"{}\",\n", py, rust));
    }
    out.push_str("    }\n");
    out.push_str(
        "    \"\"\"Python field name -> Rust canonical name (only for names that differ).\"\"\"\n\n",
    );

    // _SENSITIVE_PARAMS
    out.push_str("    _SENSITIVE_PARAMS: ClassVar[frozenset[str]] = frozenset({\n");
    for s in &sensitive {
        out.push_str(&format!("        \"{}\",\n", s));
    }
    out.push_str("    })\n");
    out.push_str("    \"\"\"Fields that contain secrets (for log redaction).\"\"\"\n\n");

    // Python-only field names — kept in one place so _PYTHON_ONLY and _ALL_FIELDS
    // stay in sync.  See the dataclass field block above for the matching defs.
    let python_only_fields = [
        "session_parameters",
        "application",
        "numpy",
        "arrow_number_to_decimal",
        "paramstyle",
        "autocommit",
        "connections_file_path",
        "auto_cleanup",
        "converter_class",
    ];

    // _PYTHON_ONLY
    out.push_str("    _PYTHON_ONLY: ClassVar[frozenset[str]] = frozenset({\n");
    for name in &python_only_fields {
        out.push_str(&format!("        \"{}\",\n", name));
    }
    out.push_str("    })\n");
    out.push_str("    \"\"\"Fields handled only in Python, not forwarded to Rust.\"\"\"\n\n");

    // Collect all known field names for from_kwargs (skip statement-scoped).
    let mut all_field_names: Vec<String> = params
        .iter()
        .filter(|p| p.scope != ParamScope::Statement)
        .map(|p| to_python_field(p.canonical_name))
        .collect();
    for name in &python_only_fields {
        all_field_names.push(name.to_string());
    }
    all_field_names.sort();

    // _ALL_FIELDS
    out.push_str("    _ALL_FIELDS: ClassVar[frozenset[str]] = frozenset({\n");
    for name in &all_field_names {
        out.push_str(&format!("        \"{}\",\n", name));
    }
    out.push_str("    })\n");
    out.push_str("    \"\"\"All known field names (PARAM_DEF + Python-only).\"\"\"\n\n");

    // ── Legacy rewrite map ─────────────────────────────────────────────
    out.push_str("    _LEGACY_REWRITES: ClassVar[dict[str, str]] = {\n");
    out.push_str("        \"private_key_file_pwd\": \"private_key_password\",\n");
    out.push_str("        \"client_request_mfa_token\": \"client_store_temporary_credential\",\n");
    out.push_str("    }\n");
    out.push_str("    \"\"\"Legacy parameter names -> canonical replacements.\"\"\"\n\n");

    out.push_str("    _APPLICATION_NAME: ClassVar[str] = \"PythonConnector\"\n");
    out.push_str("    \"\"\"Default application name.\"\"\"\n\n");

    out.push_str("    _APPLICATION_RE: ClassVar[re.Pattern[str]] = re.compile(r\"^[\\w\\d_]+\")\n");
    out.push_str("    \"\"\"Regex for validating application names.\"\"\"\n\n");

    // ── from_kwargs classmethod ────────────────────────────────────────
    out.push_str(
        r#"    @classmethod
    def from_kwargs(cls, **kwargs: Any) -> ConnectionConfig:
        """Create a ConnectionConfig from keyword arguments.

        Resolves case-insensitive aliases, applies legacy parameter name
        rewrites, and collects unknown keys into ``_extra``.
        """
        # Apply legacy rewrites first
        for old_name, new_name in cls._LEGACY_REWRITES.items():
            if old_name in kwargs:
                value = kwargs.pop(old_name)
                if new_name not in kwargs:
                    kwargs[new_name] = value

        resolved: dict[str, Any] = {}
        extra: dict[str, Any] = {}

        for key, value in kwargs.items():
            lower_key = key.lower()
            # Direct field match (already snake_case)
            if lower_key in cls._ALL_FIELDS:
                resolved[lower_key] = value
            # Alias match
            elif lower_key in cls._ALIAS_MAP:
                field_name = cls._ALIAS_MAP[lower_key]
                resolved[field_name] = value
            else:
                extra[key] = value

        config = cls(**resolved)
        config._extra = extra
        return config

"#,
    );

    // ── from_connection_args classmethod ────────────────────────────────
    out.push_str(
        r#"    @classmethod
    def from_connection_args(
        cls,
        connection_name: str | None = None,
        connections_file_path: str | None = None,
        config: ConnectionConfig | None = None,
        **kwargs: Any,
    ) -> ConnectionConfig:
        """Build a ConnectionConfig from Connection constructor arguments.

        Merges ``connection_name``, ``connections_file_path``, an optional
        pre-built ``config``, and any remaining ``**kwargs`` into a single
        :class:`ConnectionConfig`.  Raises ``ProgrammingError`` when both *config*
        and *kwargs* are supplied.

        Performs all value normalisation and validation so that the returned
        config is ready to be consumed by ``to_options()`` without further
        processing:

        * ``private_key`` – normalises RSAPrivateKey / bytes / str via
          :func:`normalize_private_key`.
        * ``application`` – validates against ``_APPLICATION_RE``, defaults to
          ``_APPLICATION_NAME``, and injects ``client_app_id`` into
          ``_extra`` for the Rust core.
        * ``autocommit`` – type-checked (must be ``bool``), then merged into
          ``session_parameters["AUTOCOMMIT"]``.
        """
        if config is not None and kwargs:
            raise ProgrammingError(
                "Cannot pass both a ConnectionConfig object and keyword arguments. "
                "Use one or the other."
            )

        if config is None:
            if connection_name is not None:
                kwargs["connection_name"] = connection_name
            if connections_file_path is not None:
                kwargs["connections_file_path"] = connections_file_path
            config = cls.from_kwargs(**kwargs)
        else:
            # config was provided — apply explicit overrides
            if connection_name is not None:
                config.connection_name = connection_name
            if connections_file_path is not None:
                config.connections_file_path = connections_file_path

        # -- private_key normalisation ------------------------------------
        if config.private_key is not None:
            config.private_key = normalize_private_key(config.private_key)

        # -- application validation & defaulting --------------------------
        application = config.application
        if application is None or (isinstance(application, str) and not application):
            config.application = cls._APPLICATION_NAME
        elif isinstance(application, str):
            if not cls._APPLICATION_RE.match(application):
                raise ProgrammingError(f"Invalid application name: {application!r}")
        else:
            raise ProgrammingError(
                f"Invalid application parameter (must be a non-empty string): {application!r}"
            )
        # Inject client_app_id for the Rust core (always present in to_options via _extra)
        config._extra["client_app_id"] = config.application

        # -- autocommit validation & session_parameters injection ---------
        if config.autocommit is not None:
            if not isinstance(config.autocommit, bool):
                raise ProgrammingError(f"Invalid autocommit parameter: {config.autocommit!r}")
            if config.session_parameters is None:
                config.session_parameters = {}
            config.session_parameters["AUTOCOMMIT"] = str(config.autocommit).lower()

        return config

"#,
    );

    // ── to_options method ──────────────────────────────────────────────
    out.push_str(
        r#"    def to_options(
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

        for field_name in self._ALL_FIELDS:
            if field_name in self._PYTHON_ONLY:
                continue
            value = getattr(self, field_name, None)
            if value is None:
                continue
            # Map back to Rust canonical name if different
            rust_name = self._PYTHON_TO_RUST_NAME.get(field_name, field_name)
            options[rust_name] = value

        # Include unknown params for Rust-side validation
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
        return {
            k: ("***" if k in self._SENSITIVE_PARAMS else v)
            for k, v in self.to_options(options_modifiers).items()
        }

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
            # Order matters: ``bool`` is a subclass of ``int`` in Python, so
            # the bool check has to come first to avoid coercing ``True`` /
            # ``False`` into ``int_value``.
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
"#,
    );

    out
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    let params = registry().all_params();
    let source = generate(params);
    print!("{}", source);
}
