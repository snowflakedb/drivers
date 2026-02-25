use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, parse_macro_input};

/// Derive macro for the `ErrorTrace` trait.
///
/// Generates an implementation that walks the snafu error enum variants,
/// collecting `ErrorTraceEntry` values from each level's `location` field
/// and display message, then recursing into `source` fields via the
/// autoref-based specialization hack.
#[proc_macro_derive(ErrorTrace)]
pub fn error_trace_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match error_trace_derive_inner(&input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error().into(),
    }
}

fn error_trace_derive_inner(input: &DeriveInput) -> syn::Result<TokenStream> {
    let enum_name = &input.ident;

    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => {
            return Err(syn::Error::new_spanned(
                enum_name,
                "ErrorTrace can only be derived for enums",
            ));
        }
    };

    let mut match_arms = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;

        let fields = match &variant.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    variant_name,
                    format!("ErrorTrace: variant `{variant_name}` must have named fields"),
                ));
            }
        };

        let has_location = fields
            .iter()
            .any(|f| f.ident.as_ref().is_some_and(|name| name == "location"));

        let source_field: Option<&Ident> = fields.iter().find_map(|f| {
            let name = f.ident.as_ref()?;
            if name == "source" { Some(name) } else { None }
        });

        if !has_location {
            return Err(syn::Error::new_spanned(
                variant_name,
                format!("ErrorTrace: variant `{variant_name}` is missing a `location` field"),
            ));
        }

        let arm = if source_field.is_some() {
            quote! {
                Self::#variant_name { location, source, .. } => {
                    let mut trace = ::std::vec![::error_trace::ErrorTraceEntry {
                        location: ::error_trace::Location::from(*location),
                        message: ::std::string::ToString::to_string(&self),
                    }];
                    trace.extend(
                        ::error_trace::ErrorTraceResolver(source).resolve()
                    );
                    trace
                }
            }
        } else {
            quote! {
                Self::#variant_name { location, .. } => {
                    ::std::vec![::error_trace::ErrorTraceEntry {
                        location: ::error_trace::Location::from(*location),
                        message: ::std::string::ToString::to_string(&self),
                    }]
                }
            }
        };
        match_arms.push(arm);
    }

    let expanded = quote! {
        impl ::error_trace::ErrorTrace for #enum_name {
            fn error_trace(&self) -> ::std::vec::Vec<::error_trace::ErrorTraceEntry> {
                use ::error_trace::ErrorTraceFallback as _;
                match self {
                    #( #match_arms ),*
                }
            }
        }
    };

    Ok(TokenStream::from(expanded))
}
