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
    let enum_name = &input.ident;

    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => panic!("ErrorTrace can only be derived for enums"),
    };

    let match_arms = variants.iter().map(|variant| {
        let variant_name = &variant.ident;

        let fields = match &variant.fields {
            Fields::Named(named) => &named.named,
            _ => panic!(
                "ErrorTrace: variant `{}` must have named fields",
                variant_name
            ),
        };

        let has_location = fields
            .iter()
            .any(|f| f.ident.as_ref().is_some_and(|name| name == "location"));

        let source_field: Option<&Ident> = fields.iter().find_map(|f| {
            let name = f.ident.as_ref()?;
            if name == "source" { Some(name) } else { None }
        });

        if !has_location {
            panic!(
                "ErrorTrace: variant `{}` is missing a `location` field",
                variant_name
            );
        }

        if let Some(_source_ident) = source_field {
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
        }
    });

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

    TokenStream::from(expanded)
}
