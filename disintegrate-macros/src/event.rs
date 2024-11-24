mod stream;

use proc_macro2::TokenStream;
use quote::quote;
use stream::{impl_stream, streams};
use syn::{AngleBracketedGenericArguments, Data, DeriveInput, Error, Result};
use syn::{DataEnum, DataStruct, Fields};

use crate::reserved_identifier_names;
use crate::symbol::ID;

pub fn event_inner(ast: &DeriveInput) -> Result<TokenStream> {
    match ast.data {
        Data::Enum(ref data) => {
            let derive_event = impl_enum(ast, data)?;
            let streams = streams(ast)?;
            let impl_streams = streams
                .iter()
                .map(|g| impl_stream(ast, g))
                .collect::<Result<Vec<TokenStream>>>()?;
            let derive_event_streams = streams
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
                  #(#impl_streams)*
                  #(#derive_event_streams)*
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
                    let payload_field = fields.unnamed.first().unwrap();
                    let payload_type = enum_unnamed_field_type(payload_field);
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

    let events = data
        .variants
        .iter()
        .map(|variant| variant.ident.to_string());

    let events_info= data
        .variants
        .iter()
        .fold(quote!(&[]), |acc, variant| {
           let variant_ident = &variant.ident.to_string();
            match &variant.fields {
            Fields::Unnamed(fields) => {
                let payload_field = fields.unnamed.first().unwrap();
                let payload_type = enum_unnamed_field_type(payload_field);
                quote! {
                    {
                        const EVENT_INFO: &[&disintegrate::EventInfo] = {
                            if #payload_type::SCHEMA.events_info.len() != 1 {
                                panic!(concat!("Event variant ", #variant_ident, " must contain a struct"));
                            }
                            &[&disintegrate::EventInfo{name: #variant_ident, domain_identifiers: #payload_type::SCHEMA.events_info[0].domain_identifiers}]
                        };
                        disintegrate::const_slices_concat!(
                            &disintegrate::EventInfo,
                            #acc,
                            EVENT_INFO
                        )
                    }
                }
            }
            Fields::Named(fields) => {
                let identifiers_idents: Vec<_> = fields
                    .named
                    .iter()
                    .filter(|f| f.attrs.iter().any(|attr| attr.path() == ID))
                    .map(|f| f.ident.as_ref())
                    .collect();
                quote! {
                    disintegrate::const_slices_concat!(&disintegrate::EventInfo, #acc, &[&disintegrate::EventInfo{name: #variant_ident, domain_identifiers: &[#(&disintegrate::ident!(##identifiers_idents),)*]}])
                }
            }
            Fields::Unit => quote!(
                disintegrate::const_slices_concat!(&disintegrate::EventInfo, #acc, &[&disintegrate::EventInfo{name: #variant_ident, domain_identifiers: &[]}])
            ),
        }});

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
        #[automatically_derived]
        impl disintegrate::Event for #name {
            const SCHEMA: disintegrate::EventSchema = disintegrate::EventSchema {
                events: &[#(#events,)*],
                events_info: #events_info,
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

fn enum_unnamed_field_type(payload_field: &syn::Field) -> &syn::Type {
    if let syn::Type::Path(ref ty_path) = payload_field.ty {
        let last_segment = ty_path.path.segments.last().expect("one path segment");
        if last_segment.ident == "Box" {
            match last_segment.arguments {
                syn::PathArguments::AngleBracketed(AngleBracketedGenericArguments {
                    ref args,
                    ..
                }) => match args.last().unwrap() {
                    syn::GenericArgument::Type(boxed_type) => return boxed_type,
                    _ => unreachable!("box should have a type generic argument"),
                },
                _ => unreachable!("box should have a bracketed generic argument"),
            }
        }
    };
    &payload_field.ty
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
        #[automatically_derived]
        impl disintegrate::Event for #name {
            const SCHEMA: disintegrate::EventSchema = disintegrate::EventSchema{
                events: &[#impl_type],
                events_info: &[&disintegrate::EventInfo{name: #impl_type, domain_identifiers: &[#(&disintegrate::ident!(##identifiers_idents),)*]}],
                domain_identifiers:&[#(&disintegrate::DomainIdentifierInfo{ident: disintegrate::ident!(##identifiers_idents), type_info: <#identifiers_types as disintegrate::IntoIdentifierValue>::TYPE},)*]
            };

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
