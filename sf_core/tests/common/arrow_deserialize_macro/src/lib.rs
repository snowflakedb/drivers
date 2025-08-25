// lib.rs of the procedural macro crate

use proc_macro::TokenStream;
use quote::quote;
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

    // Generate the struct instantiation code for a specific row using ArrowExtractValue trait
    let struct_field_initializers = fields.iter().enumerate().map(|(i, field)| {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;

        quote! {
            #field_name: crate::common::arrow_extract_value::extract_arrow_value::<#field_type>(
                batch.column(#i),
                row_index
            ).map_err(|e| format!("Failed to extract value for field '{}': {:?}", stringify!(#field_name), e))?
        }
    });

    // Assemble the final implementation using the generated code blocks.
    let expanded = quote! {
        // Implement the ArrowDeserialize trait for the user's struct.
        impl crate::common::arrow_deserialize::ArrowDeserialize for #struct_name {
            fn deserialize_one(batch: &crate::common::arrow_deserialize::RecordBatch, row_index: usize) -> Result<Self, String> {
                // Import necessary types into the generated code's scope.
                use crate::common::arrow_extract_value::{ArrowExtractValue, extract_arrow_value};

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

                // Create and return the struct instance for the specified row.
                Ok(Self {
                    // Generate field initializers using ArrowExtractValue trait.
                    #( #struct_field_initializers ),*
                })
            }
        }
    };

    // Convert the generated `quote` into a `TokenStream` and return it.
    TokenStream::from(expanded)
}
