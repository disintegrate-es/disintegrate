mod group;

use group::{groups, impl_group};
use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::quote;
use syn::{Data, DeriveInput};
use syn::{DataEnum, DataStruct, Fields};

pub fn event_inner(ast: &DeriveInput) -> TokenStream {
    match ast.data {
        Data::Enum(ref data) => {
            let derive_event = impl_enum(ast, data);
            let groups = groups(ast);
            let impl_groups = groups.iter().map(|g| impl_group(ast, g));
            let derive_event_groups = groups.iter().map(|g| {
                if let Data::Enum(ref enum_data) = g.data {
                    impl_enum(g, enum_data)
                } else {
                    panic!("Expect to be an enum data");
                }
            });

            let res = quote! {
                #derive_event
                #(#impl_groups)*
                #(#derive_event_groups)*
            };

            res.into()
        }
        Data::Struct(ref data) => impl_struct(ast, data).into(),
        _ => panic!("Not supported type"),
    }
}

fn impl_enum(ast: &DeriveInput, data: &DataEnum) -> TokenStream2 {
    let name = ast.ident.clone();
    let impl_name = data.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let event_name = variant_ident.to_string();

        quote! {
            #name::#variant_ident{ .. } => #event_name,
        }
    });

    let impl_domain_identifiers = data.variants.iter().map(|variant| {
        let event_type = &variant.ident;

        match &variant.fields {
            Fields::Unnamed(_fields) => quote!{
                  #name::#event_type(payload) => payload.domain_identifiers(),
            },
            Fields::Named(fields) => {
                let identifiers_fields : Vec<_> = fields.named
                    .iter()
                    .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("id")))
                    .map(|f| f.ident.as_ref())
                    .collect();

                let reserved_identifiers = reserved_identifier_names(&identifiers_fields);
                quote! {
                    #name::#event_type{#(#identifiers_fields,)*..} => {
                        #reserved_identifiers
                        disintegrate::domain_identifiers!{#(#identifiers_fields: #identifiers_fields),*}
                    },
                }
            },
            Fields::Unit => quote! {
                     #name::#event_type => disintegrate::domain_identifiers!{},
            }
        }

    });

    let domain_identifiers_slice =
        data.variants
            .iter()
            .fold(quote!(&[]), |acc, variant| match &variant.fields {
                Fields::Unnamed(fields) => {
                    let payload_type = &fields.unnamed.first().unwrap().ty;
                    quote! {
                        disintegrate::const_slices_concat!(
                            #acc,
                            #payload_type::SCHEMA.domain_identifiers
                        )
                    }
                }
                Fields::Named(fields) => {
                    let identifiers_fields: Vec<_> = fields
                        .named
                        .iter()
                        .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("id")))
                        .map(|f| f.ident.as_ref().map(ToString::to_string))
                        .collect();
                    quote! {
                        disintegrate::const_slices_concat!(#acc, &[#(#identifiers_fields,)*])
                    }
                }
                Fields::Unit => quote!(disintegrate::const_slices_concat!(#acc, &[])),
            });

    let types = data
        .variants
        .iter()
        .map(|variant| variant.ident.to_string());

    let impl_domain_identifiers_schema = quote! {
        disintegrate::const_slice_unique!(#domain_identifiers_slice)
    };
    quote! {
        impl disintegrate::Event for #name {
            const SCHEMA: disintegrate::EventSchema = disintegrate::EventSchema {
                types: &[#(#types,)*],
                domain_identifiers: #impl_domain_identifiers_schema,
            };

            fn name(&self) -> &'static str {
                match self {
                   #(#impl_name)*
                }
            }

            fn domain_identifiers(&self) -> disintegrate::DomainIdentifierSet {
                match self {
                    #(#impl_domain_identifiers)*
                 }
            }
        }
    }
}

fn impl_struct(ast: &DeriveInput, data: &DataStruct) -> TokenStream2 {
    let name = ast.ident.clone();
    let impl_type = name.to_string();

    let identifiers_fields: Vec<_> = data
        .fields
        .iter()
        .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("id")))
        .map(|f| f.ident.as_ref())
        .collect();

    let identifiers_names: Vec<_> = identifiers_fields
        .iter()
        .map(|f| f.map(ToString::to_string))
        .collect();

    let reserved_identifiers = reserved_identifier_names(&identifiers_fields);

    quote! {
        impl disintegrate::Event for #name {

            const SCHEMA: disintegrate::EventSchema = disintegrate::EventSchema{types: &[#impl_type], domain_identifiers: &[#(#identifiers_names,)*]};

            fn name(&self) -> &'static str {
                #impl_type
            }

            fn domain_identifiers(&self) -> disintegrate::DomainIdentifierSet {
                #reserved_identifiers
                disintegrate::domain_identifiers!{#(#identifiers_fields: self.#identifiers_fields),*}
            }
        }
    }
}

fn reserved_identifier_names(identifiers_fields: &[Option<&Ident>]) -> Option<TokenStream2> {
    const RESERVED_NAMES: &[&str] = &["event_id", "payload", "event_type", "inserted_at"];

    if let Some(Some(identifier)) = identifiers_fields.iter().find(|id| {
        id.map_or(false, |id| {
            RESERVED_NAMES.contains(&id.to_string().as_str())
        })
    }) {
        return Some(
            syn::Error::new(
                identifier.span(),
                "Reserved domain identifier name. Please use a different name",
            )
            .to_compile_error(),
        );
    }
    None
}
