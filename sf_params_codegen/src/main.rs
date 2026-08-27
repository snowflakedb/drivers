//! Generates the Python `ConnectionConfig` dataclass from the canonical
//! `PARAM_DEFS` registry.
//!
//! The output is a ready-to-use Python source file printed to stdout and
//! consumed by the `generate-connection-config` pre-commit hook whenever the
//! `sf_params_spec` registry or this generator changes.
//!
//! Only the *static* information that can be derived directly from
//! `PARAM_DEFS` is emitted here:
//!
//! * the PARAM_DEFS-backed dataclass fields (types, defaults, docstrings); and
//! * the lookup tables that depend on those fields — `_ALIAS_MAP`,
//!   `_PYTHON_TO_RUST_NAME`, `_SENSITIVE_PARAMS`.
//!
//! Everything behavioural — Python-only fields, `_extra`, legacy rewrites,
//! application validation, `from_kwargs` / `to_options` / `to_proto_options`
//! — lives in the hand-written
//! `snowflake.connector._internal.connection_config_mixin.ConnectionConfigMixin`
//! that the generated class inherits from.

use sf_params_spec::{DefaultValue, ParamDef, ParamScope, Required, ValueType, Wrapper, registry};

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

/// Format a [`DefaultValue`] as a Python literal that is safe to paste into
/// generated source code.
///
/// String / byte values escape backslashes, single quotes, and any character
/// that would otherwise terminate the literal or render it ambiguous (control
/// chars, non-graphic bytes).  This matters because callers occasionally
/// embed the result back into the generated Python (e.g. `Default: ...` in
/// docstrings); a bare `\` or `'` would break the surrounding triple-quoted
/// literal otherwise.
fn default_value_to_py_literal(d: &DefaultValue) -> String {
    match d {
        DefaultValue::String(v) => {
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
        DefaultValue::Int(v) => v.to_string(),
        DefaultValue::Double(v) => v.to_string(),
        DefaultValue::Bool(v) => {
            if *v {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        DefaultValue::Bytes(v) => {
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

/// Word-wrap `text` so that no output line exceeds `max_cols` characters
/// (including the indentation prefix).  The first line gets
/// `first_line_budget` characters; continuation lines are prefixed with
/// `continuation_indent` and get `max_cols - continuation_indent.len()`
/// characters.
fn wrap_text(
    text: &str,
    first_line_budget: usize,
    continuation_indent: &str,
    max_cols: usize,
) -> String {
    let cont_budget = max_cols - continuation_indent.len();
    let mut result = String::new();
    let mut budget = first_line_budget;
    let mut first_word = true;

    for word in text.split_whitespace() {
        let needed = if first_word {
            word.len()
        } else {
            1 + word.len()
        };
        if !first_word && needed > budget {
            result.push('\n');
            result.push_str(continuation_indent);
            budget = cont_budget - word.len();
            result.push_str(word);
        } else {
            if !first_word {
                result.push(' ');
            }
            result.push_str(word);
            budget = budget.saturating_sub(needed);
            first_word = false;
        }
    }
    result
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

/// Representative scope for grouping a possibly multi-scoped parameter, by
/// precedence Connection > Session > Statement.
fn primary_scope(scopes: &[ParamScope]) -> ParamScope {
    if scopes.contains(&ParamScope::Connection) {
        ParamScope::Connection
    } else if scopes.contains(&ParamScope::Session) {
        ParamScope::Session
    } else {
        ParamScope::Statement
    }
}

/// A parameter belongs on the connection config unless it is statement-only.
fn is_statement_only(scopes: &[ParamScope]) -> bool {
    !scopes.contains(&ParamScope::Connection) && !scopes.contains(&ParamScope::Session)
}

/// Wrapper-specific params excluded from the generated Python
/// `ConnectionConfig` — they stay in `PARAM_DEFS` for the wrappers that own
/// them but Python never exposed them:
///   - `put_fastfail` / `get_fastfail` — ODBC connection-string pipeline
///   - `enable_put_get` — JDBC-only (legacy `enablePutGet` client property)
const PYTHON_EXCLUDED_PARAMS: &[&str] = &["put_fastfail", "get_fastfail", "enable_put_get"];

/// Whether `canonical` is intentionally excluded from the generated Python
/// `ConnectionConfig` (see [`PYTHON_EXCLUDED_PARAMS`]).
fn is_python_excluded(canonical: &str) -> bool {
    PYTHON_EXCLUDED_PARAMS.contains(&canonical)
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
        (scope_section(primary_scope(a.scopes)), a.canonical_name)
            .cmp(&(scope_section(primary_scope(b.scopes)), b.canonical_name))
    });

    // ── Header ─────────────────────────────────────────────────────────
    out.push_str(
        r#""""Auto-generated from PARAM_DEFS in sf_params_spec. DO NOT EDIT.

Regenerated by the generate-connection-config pre-commit hook whenever the
sf_params_spec registry or the generator changes.  Any manual changes will be
silently overwritten.

This file intentionally contains only information derivable from
``PARAM_DEFS``: the dataclass fields and the ``_ALIAS_MAP`` /
``_PYTHON_TO_RUST_NAME`` / ``_SENSITIVE_PARAMS`` lookup tables.  Behavioural
helpers (``from_kwargs``, ``to_options``, ``to_proto_options``, legacy
rewrites, application validation, Python-only fields) live in
:mod:`snowflake.connector._internal.connection_config_mixin` and are inherited
via :class:`ConnectionConfigMixin`.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Any, ClassVar

from snowflake.connector._internal.connection_config_mixin import (
    ConnectionConfigMixin,
    OptionsModifier,
)

__all__ = ["ConnectionConfig", "OptionsModifier"]


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
        // Skip statement-only params; they belong on the cursor, not the connection.
        // (A param that is also session/connection-scoped still appears here.)
        if is_statement_only(p.scopes) || is_python_excluded(p.canonical_name) {
            continue;
        }

        let py_field = to_python_field(p.canonical_name);

        // Canonical name itself as an alias (lowercased)
        let canonical_lower = p.canonical_name.to_ascii_lowercase();
        if canonical_lower != py_field {
            alias_entries.push((canonical_lower.clone(), py_field.clone()));
        }

        for alias in p.alias_names_for(Wrapper::Python) {
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
    out.push_str("@dataclass\nclass ConnectionConfig(ConnectionConfigMixin):\n");
    out.push_str(
        "    \"\"\"Typed connection configuration derived from sf_params_spec PARAM_DEFS.\n\n",
    );
    out.push_str("    Fields use defaults from the Rust core where defined, ``None`` otherwise.\n");
    out.push_str("    Behavioural helpers (``from_kwargs``, ``to_options``, etc.) are inherited\n");
    out.push_str("    from :class:`ConnectionConfigMixin`.\n");
    out.push_str("    \"\"\"\n\n");

    // Fields grouped by scope section
    let mut current_scope: Option<ParamScope> = None;

    for p in sorted_params.iter().copied() {
        // Skip statement-only params; they belong on the cursor, not the connection.
        // (A param that is also session/connection-scoped still appears here.)
        if is_statement_only(p.scopes) || is_python_excluded(p.canonical_name) {
            continue;
        }

        if current_scope != Some(primary_scope(p.scopes)) {
            current_scope = Some(primary_scope(p.scopes));
            out.push_str(&format!(
                "    # -- {} parameters {}\n",
                scope_section(primary_scope(p.scopes)),
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

        // Use actual default from the registry when available, otherwise None
        let default_literal = if let Some(default) = p.default {
            default_value_to_py_literal(&default)
        } else {
            "None".to_string()
        };
        out.push_str(&format!(
            "    {}: {} = {}\n",
            py_field, type_ann, default_literal
        ));

        // Docstring with description and metadata
        let mut doc_parts = vec![p.description.to_string()];
        if let Some(default) = p.default {
            doc_parts.push(format!(
                "Default: {}",
                default_value_to_py_literal(&default)
            ));
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
        // Render the docstring on a single line when it fits in 120
        // columns, otherwise spread it across multiple lines so ruff's
        // E501 (line-too-long) doesn't reject the generated file.
        let single = format!("    \"\"\"{}\"\"\"\n\n", doc_parts.join(". "));
        if single.len() <= 121 {
            // 120 cols + trailing newline (\n)
            out.push_str(&single);
        } else {
            out.push_str("    \"\"\"");
            for (i, part) in doc_parts.iter().enumerate() {
                if i > 0 {
                    out.push_str(".\n\n    ");
                }
                let first_budget = if i == 0 { 120 - 7 } else { 120 - 4 };
                out.push_str(&wrap_text(part, first_budget, "    ", 120));
            }
            out.push_str("\n    \"\"\"\n\n");
        }
    }

    // ── Class variables (PARAM_DEFS-derived lookup tables) ─────────────
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
    out.push_str("    \"\"\"Fields that contain secrets (for log redaction).\"\"\"\n");

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
