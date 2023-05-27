mod group;

use group::{groups, impl_group};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Variant};
use syn::{DataEnum, DataStruct, Fields};

pub fn event_inner(ast: &DeriveInput) -> TokenStream {
    match ast.data {
        Data::Enum(ref data) => {
            let variant_prefix = event_name(ast);
            let derive_event = impl_enum(ast, data, &variant_prefix);
            let groups = groups(ast);
            let impl_groups = groups.iter().map(|g| impl_group(ast, g));
            let derive_event_groups = groups.iter().map(|g| {
                if let Data::Enum(ref enum_data) = g.data {
                    impl_enum(g, enum_data, &variant_prefix)
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

fn impl_enum(ast: &DeriveInput, data: &DataEnum, name_prefix: &str) -> TokenStream2 {
    let name = ast.ident.clone();
    let names = data
        .variants
        .iter()
        .map(|variant| variant_name(name_prefix, variant));
    let impl_name = data.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let event_name = variant_name(name_prefix, variant);

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

                quote! {
                    #name::#event_type{#(#identifiers_fields,)*..} => disintegrate::domain_identifiers!{#(#identifiers_fields: #identifiers_fields),*},
                }
            },
            Fields::Unit => quote! {
                     #name::#event_type => disintegrate::domain_identifiers!{},
            }
        }

    });

    quote! {
        impl disintegrate::Event for #name {
            const NAMES: &'static [&'static str] = &[#(#names,)*];
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
    let impl_name = event_name(ast);

    let identifiers_fields: Vec<_> = data
        .fields
        .iter()
        .filter(|f| f.attrs.iter().any(|attr| attr.path().is_ident("id")))
        .map(|f| f.ident.as_ref())
        .collect();

    quote! {
        impl disintegrate::Event for #name {
            const NAMES: &'static [&'static str] = &[#impl_name];
            fn name(&self) -> &'static str {
                #impl_name
            }

            fn domain_identifiers(&self) -> disintegrate::DomainIdentifierSet {
                disintegrate::domain_identifiers!{#(#identifiers_fields: self.#identifiers_fields),*}
            }
        }
    }
}

fn event_name(ast: &DeriveInput) -> String {
    let event_name = ast.ident.to_string();
    event_name
        .strip_suffix("Event")
        .unwrap_or(&event_name)
        .to_owned()
}

fn variant_name(prefix: &str, variant: &Variant) -> String {
    format!("{}{}", prefix, variant.ident)
}
