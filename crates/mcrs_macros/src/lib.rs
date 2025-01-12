use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Variant};

#[proc_macro_derive(EnumIter)]
pub fn enum_iter(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    match ast.data {
        Data::Enum(data_enum) => {
            let enum_name = ast.ident;
            let enum_variants: Vec<_> = data_enum
                .variants
                .iter()
                .map(|Variant { ident, .. }| ident)
                .collect();

            TokenStream::from(quote! {
                impl #enum_name {
                    pub fn iter() -> impl Iterator<Item=#enum_name> {
                        vec![#(#enum_name::#enum_variants),*].into_iter()
                    }
                }
            })
        }
        _ => panic!("EnumIter is only defined for enums."),
    }
}
