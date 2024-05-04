use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Variant};

#[proc_macro_derive(EnumIter)]
pub fn enum_iter(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    match ast.data {
        Data::Enum(data_enum) => {
            let enum_name = ast.ident;
            let variant_idents: Vec<_> = data_enum.variants.iter().map(|Variant { ident, .. }| ident).collect();

            TokenStream::from(quote!{
                impl #enum_name {
                    pub fn iter() -> impl Iterator<Item=#enum_name> {
                        vec![#(#enum_name::#variant_idents),*].into_iter()
                    }
                }
            })
        },
        _ => panic!("IterateEnumVariants is only defined for enums"),
    }
}