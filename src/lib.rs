use lazy_static::lazy_static;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use regex::Regex;
use syn::{
    parse::{Parse, ParseStream},
    FieldsUnnamed, Variant,
};
use syn::{Fields, ItemEnum};
use syn::{Ident, LitStr, Token};

lazy_static! {
    static ref PATTERN: Regex = Regex::new(r#"\{\w+\}"#).unwrap();
}

struct Route {
    method: Ident,
    url: LitStr,
}

impl Parse for Route {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = input.parse::<Ident>()?;
        input.parse::<Token![,]>()?;
        let url = input.parse::<LitStr>()?;

        Ok(Route { method, url })
    }
}

fn impl_unit(
    enum_name: &Ident,
    variant: &Variant,
    route_attribute: &Route,
) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;
    let method = &route_attribute.method;
    let route_url = &route_attribute.url;

    quote! {
        #enum_name::#variant_name => (Method::#method, #route_url.to_string())
    }
}

fn impl_unnamed_fields(
    enum_name: &Ident,
    variant: &Variant,
    fields: &FieldsUnnamed,
    route_attribute: &Route,
) -> proc_macro2::TokenStream {
    let variant_name = &variant.ident;
    let method = &route_attribute.method;
    let route_url = &route_attribute.url;
    let route_string = &route_attribute.url.value();
    let parameter_names = PATTERN
        .find_iter(&route_string)
        .map(|m| {
            let as_str = m.as_str();
            format_ident!("{}", &as_str[1..as_str.len() - 1]) // "{abc}" -> "abc"
        })
        .collect::<Vec<_>>();
    let types = fields
        .unnamed
        .iter()
        .map(|field| &field.ty)
        .collect::<Vec<_>>();

    if parameter_names.len() != types.len() {
        panic!("Must have the same amount of parameter names as types in enum field.")
    }

    quote! {
        #enum_name::#variant_name(#(#parameter_names),*) => (
            Method::#method,
            format!(
                #route_url,
                #(#parameter_names = #parameter_names),*
            )
        )
    }
}

#[proc_macro_derive(Routes, attributes(route))]
pub fn route_macro(input: TokenStream) -> TokenStream {
    let item: ItemEnum = syn::parse(input).expect("Route can only be derived for enums");
    let mut routes = Vec::new();

    if item.variants.is_empty() {
        panic!("Cannot derive `Routes` on empty enum")
    }

    let variants_with_route = item.variants.iter().filter_map(|variant| {
        variant
            .attrs
            .iter()
            .find(|attr| attr.path.is_ident("route"))
            .map(|attr| {
                let route = attr
                    .parse_args::<Route>()
                    .expect("Invalid syntax for `route` attribute");

                (variant, route)
            })
    });

    let enum_name = item.ident;

    for (variant, route_attribute) in variants_with_route {
        let impl_arm = match &variant.fields {
            Fields::Unnamed(fields) => {
                impl_unnamed_fields(&enum_name, variant, fields, &route_attribute)
            }
            Fields::Unit => impl_unit(&enum_name, variant, &route_attribute),
            _ => panic!("Fields cannot be named."),
        };

        routes.push(impl_arm)
    }

    let tokens = quote! {
        impl #enum_name {
            pub fn resolve(&self) -> (Method, String) {
                match self {
                    #(#routes),*
                }
            }
        }
    };

    tokens.into()
}
