//! Proc macros for the next-config crate.

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Expr, Lit, parse_macro_input, spanned::Spanned};

struct ConfigOptions {
    version: u32,
}

impl ConfigOptions {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut version = None;

        for attr in attrs {
            if attr.path().is_ident("config") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("version") {
                        let value: Expr = meta.value()?.parse()?;
                        if let Expr::Lit(expr_lit) = value {
                            if let Lit::Int(lit_int) = expr_lit.lit {
                                version = Some(lit_int.base10_parse::<u32>()?);
                            } else {
                                return Err(syn::Error::new(
                                    expr_lit.span(),
                                    "version must be an integer",
                                ));
                            }
                        } else {
                            return Err(syn::Error::new(value.span(), "version must be a literal"));
                        }
                    } else {
                        return Err(syn::Error::new(
                            meta.path.span(),
                            format!("unknown config attribute: {:?}", meta.path.get_ident()),
                        ));
                    }
                    Ok(())
                })?;
            }
        }

        let version = version.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "missing required attribute: #[config(version = ...)]",
            )
        })?;

        if version == 0 {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "config version must be greater than zero",
            ));
        }

        Ok(Self { version })
    }
}

/// Implements `next_config::Config` using `#[config(version = ...)]`.
#[proc_macro_derive(Config, attributes(config))]
pub fn derive_config(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    match derive_config_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn derive_config_impl(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let options = ConfigOptions::from_attrs(&input.attrs)?;
    let name = &input.ident;
    let version = options.version;

    Ok(quote! {
        impl ::next_config::Config for #name {
            const VERSION: u32 = #version;
        }
    })
}
