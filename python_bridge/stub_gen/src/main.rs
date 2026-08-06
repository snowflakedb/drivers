//! Generate `sf_core_python.pyi` by parsing `lib.rs` with `syn`.
//!
//! Inspects `#[pyfunction]`-annotated functions and maps Rust types to Python
//! type annotations. Zero runtime dependencies beyond `syn` and `std`.

use std::fmt::Write as _;
use std::fs;
use std::path::PathBuf;

use syn::{FnArg, ItemFn, Pat, ReturnType, Type, TypePath, TypeReference, TypeTuple};

fn main() {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../python/src/snowflake/connector/_core")
        });

    let lib_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../src/lib.rs");
    let source = fs::read_to_string(&lib_path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", lib_path.display()));

    let file = syn::parse_file(&source).expect("failed to parse lib.rs");

    let mut stub = String::from(
        "\"\"\"Type stubs for the sf_core_python native extension (auto-generated).\"\"\"\n\n",
    );

    for item in &file.items {
        let syn::Item::Fn(func) = item else { continue };
        if !has_pyfunction_attr(func) {
            continue;
        }
        write_stub_function(&mut stub, func);
    }

    fs::create_dir_all(&out_dir).unwrap();
    let dest = out_dir.join("sf_core_python.pyi");
    fs::write(&dest, &stub).unwrap_or_else(|e| panic!("cannot write {}: {e}", dest.display()));
    println!("wrote {dest}", dest = dest.display());
}

fn has_pyfunction_attr(func: &ItemFn) -> bool {
    func.attrs.iter().any(|a| a.path().is_ident("pyfunction"))
}

fn write_stub_function(out: &mut String, func: &ItemFn) {
    let name = &func.sig.ident;

    // Collect doc comments for the docstring.
    let docs: Vec<String> = func
        .attrs
        .iter()
        .filter_map(|a| {
            if !a.path().is_ident("doc") {
                return None;
            }
            if let syn::Meta::NameValue(nv) = &a.meta
                && let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
            {
                return Some(s.value());
            }
            None
        })
        .collect();

    // Gather parameters, skipping `py: Python` / `_py: Python`.
    let params: Vec<String> = func
        .sig
        .inputs
        .iter()
        .filter_map(|arg| {
            let FnArg::Typed(pat_type) = arg else {
                return None;
            };
            let param_name = match pat_type.pat.as_ref() {
                Pat::Ident(pi) => pi.ident.to_string(),
                _ => return None,
            };
            // Skip the Python GIL token parameter.
            if is_python_token(&pat_type.ty) {
                return None;
            }
            let py_type = rust_type_to_python(&pat_type.ty);
            Some(format!("{param_name}: {py_type}"))
        })
        .collect();

    let ret = match &func.sig.output {
        ReturnType::Default => "None".to_string(),
        ReturnType::Type(_, ty) => rust_type_to_python(ty),
    };

    let _ = writeln!(out, "def {name}({}) -> {ret}:", params.join(", "));

    if !docs.is_empty() {
        out.push_str("    \"\"\"");
        for (i, line) in docs.iter().enumerate() {
            let trimmed = line.strip_prefix(' ').unwrap_or(line);
            if i == 0 && !trimmed.is_empty() {
                out.push_str(trimmed);
                out.push('\n');
            } else if trimmed.is_empty() {
                out.push('\n');
            } else {
                let _ = writeln!(out, "    {trimmed}");
            }
        }
        out.push_str("    \"\"\"\n");
    }
    out.push_str("    ...\n\n");
}

fn is_python_token(ty: &Type) -> bool {
    match ty {
        Type::Path(tp) => {
            let seg = tp.path.segments.last().map(|s| s.ident.to_string());
            seg.as_deref() == Some("Python")
        }
        Type::Reference(TypeReference { elem, .. }) => is_python_token(elem),
        _ => false,
    }
}

fn rust_type_to_python(ty: &Type) -> String {
    match ty {
        Type::Path(tp) => path_to_python(tp),
        Type::Reference(TypeReference { elem, .. }) => rust_type_to_python(elem),
        Type::Tuple(TypeTuple { elems, .. }) => {
            if elems.is_empty() {
                "None".to_string()
            } else {
                let inner: Vec<String> = elems.iter().map(rust_type_to_python).collect();
                format!("tuple[{}]", inner.join(", "))
            }
        }
        _ => "object".to_string(),
    }
}

fn path_to_python(tp: &TypePath) -> String {
    let seg = tp.path.segments.last().unwrap();
    let ident = seg.ident.to_string();
    match ident.as_str() {
        "u8" | "u16" | "u32" | "u64" | "i8" | "i16" | "i32" | "i64" | "usize" | "isize" => {
            "int".to_string()
        }
        "f32" | "f64" => "float".to_string(),
        "bool" => "bool".to_string(),
        "str" | "String" => "str".to_string(),
        "PyBytes" | "Bytes" => "bytes".to_string(),
        "PyAny" => "object".to_string(),
        "Vec" => {
            if let Some(inner) = extract_generic_arg(seg) {
                let inner_py = rust_type_to_python(&inner);
                if inner_py == "int" {
                    // Vec<u8> → bytes
                    return "bytes".to_string();
                }
                format!("list[{inner_py}]")
            } else {
                "list[object]".to_string()
            }
        }
        "Option" => {
            if let Some(inner) = extract_generic_arg(seg) {
                let inner_py = rust_type_to_python(&inner);
                format!("{inner_py} | None")
            } else {
                "object | None".to_string()
            }
        }
        "Py" => {
            // Py<PyAny> → object, Py<PyBytes> → bytes
            if let Some(inner) = extract_generic_arg(seg) {
                rust_type_to_python(&inner)
            } else {
                "object".to_string()
            }
        }
        "Bound" => {
            // Bound<'_, PyBytes> → bytes
            if let Some(inner) = extract_last_generic_arg(seg) {
                rust_type_to_python(&inner)
            } else {
                "object".to_string()
            }
        }
        _ => "object".to_string(),
    }
}

fn extract_generic_arg(seg: &syn::PathSegment) -> Option<Type> {
    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
        for arg in &ab.args {
            if let syn::GenericArgument::Type(t) = arg {
                return Some(t.clone());
            }
        }
    }
    None
}

fn extract_last_generic_arg(seg: &syn::PathSegment) -> Option<Type> {
    if let syn::PathArguments::AngleBracketed(ab) = &seg.arguments {
        for arg in ab.args.iter().rev() {
            if let syn::GenericArgument::Type(t) = arg {
                return Some(t.clone());
            }
        }
    }
    None
}
