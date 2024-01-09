mod group;

use group::{groups, impl_group};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Result};
use syn::{DataEnum, DataStruct, Fields};

use crate::reserved_identifier_names;
use crate::symbol::ID;

pub fn event_inner(ast: &DeriveInput) -> Result<TokenStream> {
    match ast.data {
        Data::Enum(ref data) => {
            let derive_event = impl_enum(ast, data)?;
            let groups = groups(ast)?;
            let impl_groups = groups
                .iter()
                .map(|g| impl_group(ast, g))
                .collect::<Result<Vec<TokenStream>>>()?;
            let derive_event_groups = groups
                .iter()
                .map(|g| {
                    if let Data::Enum(ref enum_data) = g.data {
                        impl_enum(g, enum_data)
                    } else {
                        Err(Error::new(g.ident.span(), "Expect to be an enum"))
                    }
                })
                .collect::<Result<Vec<TokenStream>>>()?;

            Ok(quote! {
                  #derive_event
                  #(#impl_groups)*
                  #(#derive_event_groups)*
            })
        }
        Data::Struct(ref data) => impl_struct(ast, data),
        _ => panic!("Not supported type"),
    }
}

fn impl_enum(ast: &DeriveInput, data: &DataEnum) -> Result<TokenStream> {
    let name = ast.ident.clone();
    let no_variants_deref = if data.variants.is_empty() {
        quote!(*)
    } else {
        quote!()
    };
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
                    .filter(|f| f.attrs.iter().any(|attr| attr.path() == ID))
                    .flat_map(|f| f.ident.as_ref())
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
                            &disintegrate::DomainIdentifierInfo,
                            #acc,
                            #payload_type::SCHEMA.domain_identifiers
                        )
                    }
                }
                Fields::Named(fields) => {
                    let identifiers_fields =  fields
                        .named
                        .iter()
                        .filter(|f| f.attrs.iter().any(|attr| attr.path() == ID));

                    let identifiers_idents: Vec<_> = identifiers_fields.clone()
                        .map(|f| f.ident.as_ref())
                        .collect();

                    let identifiers_types: Vec<_> = identifiers_fields
                        .map(|f| f.ty.clone())
                        .collect();

                    quote! {
                        disintegrate::const_slices_concat!(&disintegrate::DomainIdentifierInfo, #acc, &[#(&disintegrate::DomainIdentifierInfo{ident: disintegrate::ident!(##identifiers_idents), type_info: <#identifiers_types as disintegrate::IntoIdentifierValue>::TYPE},)*])
                    }
                }
                Fields::Unit => quote!(disintegrate::const_slices_concat!(&disintegrate::DomainIdentifierInfo, #acc, &[])),
            });

    let types = data
        .variants
        .iter()
        .map(|variant| variant.ident.to_string());

    let impl_domain_identifiers_schema = quote! {
        disintegrate::const_slice_unique!(&disintegrate::DomainIdentifierInfo, #domain_identifiers_slice, const fn compare(a: &disintegrate::DomainIdentifierInfo, b: &disintegrate::DomainIdentifierInfo) -> i8 {
           let result = disintegrate::utils::compare(a.ident.into_inner(), b.ident.into_inner());
           if result == 0 && (a.type_info as isize) != (b.type_info as isize) {
            panic!("Domain identifiers must have a consistent type in all its definitions");
           }
           result
        })
    };
    Ok(quote! {
        impl disintegrate::Event for #name {
            const SCHEMA: disintegrate::EventSchema = disintegrate::EventSchema {
                types: &[#(#types,)*],
                domain_identifiers: #impl_domain_identifiers_schema,
            };

            fn name(&self) -> &'static str {
                match #no_variants_deref self {
                   #(#impl_name)*
                }
            }

            fn domain_identifiers(&self) -> disintegrate::DomainIdentifierSet {
                match #no_variants_deref self {
                    #(#impl_domain_identifiers)*
                 }
            }
        }
    })
}

fn impl_struct(ast: &DeriveInput, data: &DataStruct) -> Result<TokenStream> {
    let name = ast.ident.clone();
    let impl_type = name.to_string();

    let identifiers_fields = data
        .fields
        .iter()
        .filter(|f| f.attrs.iter().any(|attr| attr.path() == ID));

    let identifiers_idents: Vec<_> = identifiers_fields
        .clone()
        .filter_map(|f| f.ident.as_ref())
        .collect();

    let identifiers_types: Vec<_> = identifiers_fields.clone().map(|f| f.ty.clone()).collect();

    let reserved_identifiers = reserved_identifier_names(&identifiers_idents);

    Ok(quote! {
        impl disintegrate::Event for #name {
            const SCHEMA: disintegrate::EventSchema = disintegrate::EventSchema{types: &[#impl_type], domain_identifiers:&[#(&disintegrate::DomainIdentifierInfo{ident: disintegrate::ident!(##identifiers_idents), type_info: <#identifiers_types as disintegrate::IntoIdentifierValue>::TYPE},)*]};

            fn name(&self) -> &'static str {
                #impl_type
            }

            fn domain_identifiers(&self) -> disintegrate::DomainIdentifierSet {
                #reserved_identifiers
                disintegrate::domain_identifiers!{#(#identifiers_idents: self.#identifiers_idents),*}
            }
        }
    })
}
