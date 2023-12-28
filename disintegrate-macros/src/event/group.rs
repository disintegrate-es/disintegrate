use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    bracketed,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    token::Comma,
    Data, DeriveInput, Error, Field, Ident, Result, Token, Type, Variant,
};

#[derive(Debug)]
pub struct QueryArgs {
    name: Ident,
    variants: Vec<Ident>,
}

impl Parse for QueryArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse::<Ident>()?;

        input.parse::<Comma>()?;

        let content;
        bracketed!(content in input);
        let variants: Punctuated<Ident, Comma> =
            content.parse_terminated(Ident::parse, Token![,])?;

        Ok(Self {
            name,
            variants: variants.into_iter().collect(),
        })
    }
}

pub fn groups(ast: &DeriveInput) -> Result<Vec<DeriveInput>> {
    ast.attrs
        .iter()
        .filter(|attr| attr.path().is_ident("group"))
        .map(|g| {
            let args: QueryArgs = g.parse_args().unwrap();
            let group_ident = args.name;
            let selected_variants: Vec<_> = args.variants;

            let event_data = match ast.data {
                Data::Enum(ref enum_data) => Ok(enum_data),
                _ => Err(Error::new(
                    group_ident.span(),
                    "Can only derive from an enum",
                )),
            }?;

            let mut group_data = event_data.clone();
            group_data.variants = event_data
                .variants
                .iter()
                .filter(|variant| {
                    selected_variants
                        .iter()
                        .any(|selected| variant.ident == *selected)
                })
                .cloned()
                .collect();

            let mut group = ast.clone();
            group.ident = group_ident;
            group.data = Data::Enum(group_data);
            group.attrs = vec![];

            Ok(group)
        })
        .collect()
}

pub fn impl_group(parent: &DeriveInput, group: &DeriveInput) -> Result<TokenStream> {
    let mut group = group.clone();
    let group_ident = &group.ident;
    let parent_ident = &parent.ident;

    let error = format_ident!("{group_ident}ConvertError");

    let group_data = match group.data {
        Data::Enum(ref mut enum_data) => Ok(enum_data),
        _ => Err(Error::new(
            group_ident.span(),
            "Can only derive from an enum",
        )),
    }?;

    group_data
        .variants
        .iter_mut()
        .for_each(|variant| match &mut variant.fields {
            syn::Fields::Named(fields) => {
                fields.named.iter_mut().for_each(|f| f.attrs = vec![]);
            }
            syn::Fields::Unnamed(_) => (),
            syn::Fields::Unit => (),
        });

    let pats: Vec<TokenStream> = group_data
        .variants
        .iter()
        .map(variant_to_unary_pat)
        .collect();

    let from_group_arms = pats
        .iter()
        .map(|pat| quote!(#group_ident::#pat => #parent_ident::#pat));

    let try_from_event_arms = pats
        .iter()
        .map(|pat| quote!(#parent_ident::#pat => std::result::Result::Ok(#group_ident::#pat)));

    let vis = &group.vis;
    let (_group_impl, group_ty, _group_where) = group.generics.split_for_impl();

    let (event_impl, event_ty, event_where) = parent.generics.split_for_impl();

    Ok(quote! {
        #[derive(Clone, Debug, PartialEq, Eq)]
        #group

        #[derive(Copy, Clone, Debug)]
        #vis struct #error;

        impl std::fmt::Display for #error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Debug::fmt(self, f)
            }
        }

        impl std::error::Error for #error {}

        #[automatically_derived]
        impl #event_impl std::convert::From<#group_ident #group_ty> for #parent_ident #event_ty #event_where {
            fn from(child: #group_ident #group_ty) -> Self {
                match child {
                    #(#from_group_arms),*
                }
            }
        }

        #[automatically_derived]
        impl #event_impl std::convert::TryFrom<#parent_ident #event_ty> for #group_ident #group_ty #event_where {
            type Error = #error;

            fn try_from(parent: #parent_ident #event_ty) -> std::result::Result<Self, Self::Error> {
                match parent {
                    #(#try_from_event_arms),*,
                    _ => std::result::Result::Err(#error)
                }
            }
        }
    })
}

fn variant_to_unary_pat(variant: &Variant) -> TokenStream {
    let ident = &variant.ident;

    match &variant.fields {
        syn::Fields::Named(named) => {
            let vars: Punctuated<Ident, Token![,]> = named.named.iter().map(snake_case).collect();
            quote!(#ident{#vars})
        }
        syn::Fields::Unnamed(unnamed) => {
            let vars: Punctuated<Ident, Token![,]> = unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(idx, _)| format_ident!("var{idx}"))
                .collect();
            quote!(#ident(#vars))
        }
        syn::Fields::Unit => quote!(#ident),
    }
}

fn snake_case(field: &Field) -> Ident {
    let ident = field.ident.as_ref().unwrap_or_else(|| {
        // No ident; the Type must be Path. Use that.
        match &field.ty {
            Type::Path(path) => path.path.get_ident().unwrap(),
            _ => unimplemented!(),
        }
    });
    Ident::new(&ident.to_string().to_snake_case(), ident.span())
}
