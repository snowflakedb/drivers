// lib.rs of the procedural macro crate

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(ArrowDeserialize)]
pub fn arrow_deserialize_derive(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Get the name of the struct the macro is attached to
    let struct_name = &input.ident;

    // Ensure the macro is used on a struct with named fields
    let fields = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(f) => &f.named,
            _ => panic!("ArrowDeserialize can only be derived for structs with named fields."),
        },
        _ => panic!("ArrowDeserialize can only be derived for structs."),
    };

    let fields_count = fields.len();

    // --- Code Generation ---

    // 1. Generate code to downcast each column array from the RecordBatch
    //    and store them in variables.
    //    e.g., `let id_array = batch.column(0).as_any().downcast_ref::<Int64Array>().unwrap();`
    let column_array_declarations = fields.iter().enumerate().map(|(i, field)| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let array_var_name = format_ident!("{}_array", field_name);

        // Map Rust types to Arrow array types (supporting only i64 and String for now)
        let array_type = match field_type {
            syn::Type::Path(p) if p.path.is_ident("i64") => quote! { arrow::array::Int64Array },
            syn::Type::Path(p) if p.path.is_ident("String") => quote! { arrow::array::StringArray },
            _ => return syn::Error::new_spanned(field_type, "Unsupported field type for ArrowDeserialize. Only i64 and String are supported currently.").to_compile_error(),
        };

        quote! {
            let #array_var_name = batch
                .column(#i)
                .as_any()
                .downcast_ref::<#array_type>()
                .ok_or_else(|| format!("Failed to downcast column '{}' to {}", stringify!(#field_name), stringify!(#array_type)))?;
        }
    });

    // 2. Generate the struct instantiation code for a specific row.
    //    e.g., `id: id_array.value(row_index),`
    //    e.g., `first_name: first_name_array.value(row_index).to_string(),`
    let struct_field_initializers = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let array_var_name = format_ident!("{}_array", field_name);

        // Generate different access logic based on type
        if let syn::Type::Path(p) = field_type {
            if p.path.is_ident("String") {
                // For strings, we need to convert the `&str` from the array to an owned `String`
                return quote! { #field_name: #array_var_name.value(row_index).to_string() };
            }
        }
        // For primitive types like i64, we can copy the value directly
        quote! { #field_name: #array_var_name.value(row_index) }
    });

    // 3. Assemble the final implementation using the generated code blocks.
    let expanded = quote! {
        // Implement the ArrowDeserialize trait for the user's struct.
        impl crate::common::arrow_deserialize::ArrowDeserialize for #struct_name {
            fn deserialize_one(batch: &crate::common::arrow_deserialize::RecordBatch, row_index: usize) -> Result<Self, String> {
                // Import necessary types into the generated code's scope.
                use arrow::array::Array;

                // Check if the number of columns in the batch matches the number of fields in the struct.
                if batch.num_columns() != #fields_count {
                    return Err(format!(
                        "Schema mismatch: expected {} columns, but batch has {}",
                        #fields_count,
                        batch.num_columns()
                    ));
                }

                // Check if the row index is within bounds
                if row_index >= batch.num_rows() {
                    return Err(format!(
                        "Row index out of bounds: {} >= {}",
                        row_index,
                        batch.num_rows()
                    ));
                }

                // Paste in the generated column array declarations.
                #( #column_array_declarations )*

                // Create and return the struct instance for the specified row.
                Ok(Self {
                    // Paste in the generated field initializers.
                    #( #struct_field_initializers ),*
                })
            }
        }
    };

    // Convert the generated `quote` into a `TokenStream` and return it.
    TokenStream::from(expanded)
}
