use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::token::Comma;
use syn::{Data, DeriveInput, Error};
use syn::{DataStruct, LitStr};

use crate::symbol::{ID, RENAME, STATE_QUERY};

enum StateQueryOptionalArgs {
    Rename(LitStr),
}

impl Parse for StateQueryOptionalArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name = input.parse::<Ident>()?;
        input.parse::<syn::token::Eq>()?;

        if name == RENAME {
            let value = input.parse::<LitStr>()?;
            return Ok(Self::Rename(value));
        }

        Err(Error::new(name.span(), "invalid argument"))
    }
}

struct StateQueryArgs {
    event: Ident,
    optional_args: Vec<StateQueryOptionalArgs>,
}

impl Parse for StateQueryArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let event = input.parse::<Ident>()?;

        let comma = input.parse::<Comma>().ok();

        let mut optional_args: Vec<StateQueryOptionalArgs> = vec![];
        if comma.is_some() {
            optional_args = input
                .parse_terminated(StateQueryOptionalArgs::parse, Comma)?
                .into_iter()
                .collect();
        }

        Ok(Self {
            event,
            optional_args,
        })
    }
}

pub fn state_query_inner(ast: &DeriveInput) -> Result<TokenStream, Error> {
    match ast.data {
        Data::Struct(ref data) => impl_struct(ast, data),
        _ => panic!("Not supported type"),
    }
}

fn impl_struct(ast: &DeriveInput, data: &DataStruct) -> syn::Result<TokenStream> {
    let state_query_ident = ast.ident.clone();

    let state_query_attrs: Vec<_> = ast
        .attrs
        .iter()
        .filter(|attr| attr.path() == STATE_QUERY)
        .collect();

    if state_query_attrs.len() != 1 {
        return Err(Error::new(
            state_query_ident.span(),
            format!("expected a `{STATE_QUERY}` attribute"),
        ));
    }

    let state_query_attrs = state_query_attrs
        .first()
        .unwrap()
        .parse_args::<StateQueryArgs>()?;
    let event_type = state_query_attrs.event;
    let state_query_name = state_query_attrs
        .optional_args
        .iter()
        .map(|attrs| {
            let StateQueryOptionalArgs::Rename(rename) = attrs;
            rename.value()
        })
        .next_back()
        .unwrap_or_else(|| state_query_ident.to_string());

    let identifiers_fields: Vec<_> = data
        .fields
        .iter()
        .filter(|f| f.attrs.iter().any(|attr| attr.path() == ID))
        .flat_map(|f| f.ident.as_ref())
        .collect();

    let state_query = impl_state_query(event_type.clone(), &identifiers_fields);

    Ok(quote! {
        #[automatically_derived]
        impl disintegrate::StateQuery for #state_query_ident {
            const NAME: &'static str = #state_query_name;

            type Event = #event_type;

            fn query<ID: disintegrate::EventId>(&self) -> disintegrate::StreamQuery<ID, Self::Event> {
                #state_query
            }
        }

        impl<ID, E> From<#state_query_ident> for disintegrate::StreamQuery<ID, E>
        where
            ID: disintegrate::EventId,
            E: disintegrate::Event + Clone, <#state_query_ident as disintegrate::StateQuery>::Event: Into<E>
         {
            fn from(state: #state_query_ident) -> Self {
                state.query().cast()
            }
        }

        impl #state_query_ident {
            pub fn exclude_events<ID: disintegrate::EventId>(&self, events: &'static [&'static str]) -> disintegrate::StreamQuery<ID, <Self as disintegrate::StateQuery>::Event> {
                self.query().exclude_events(events)
            }
        }

    })
}

fn impl_state_query(event_type: Ident, identifiers_fields: &[&Ident]) -> TokenStream {
    if identifiers_fields.is_empty() {
        quote! {
            disintegrate::query!(#event_type)
        }
    } else {
        let filters = impl_state_filters(identifiers_fields);
        quote! {
            disintegrate::query!(#event_type; #filters)
        }
    }
}

fn impl_state_filters(identifiers_fields: &[&Ident]) -> Option<TokenStream> {
    if identifiers_fields.is_empty() {
        return None;
    }

    if identifiers_fields.len() == 1 {
        Some(quote! {#(#identifiers_fields == self.#identifiers_fields)*})
    } else {
        let first = identifiers_fields[0];
        let rest = impl_state_filters(&identifiers_fields[1..]);
        Some(quote! {
             #first == self.#first, #rest
        })
    }
}
