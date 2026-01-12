//! Proc macros for the next-config crate.
//!
//! This crate provides the `#[derive(Config)]` macro

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Expr, Lit, parse_macro_input, spanned::Spanned};

/// Configuration options parsed from `#[config(...)]` attribute.
struct ConfigOptions {
    version: u32,
    file_name: String,
}

impl ConfigOptions {
    fn from_attrs(attrs: &[syn::Attribute]) -> syn::Result<Self> {
        let mut version = None;
        let mut file_name = None;

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
                    } else if meta.path.is_ident("file_name") {
                        let value: Expr = meta.value()?.parse()?;
                        if let Expr::Lit(expr_lit) = value {
                            if let Lit::Str(lit_str) = expr_lit.lit {
                                file_name = Some(lit_str.value());
                            } else {
                                return Err(syn::Error::new(
                                    expr_lit.span(),
                                    "file_name must be a string",
                                ));
                            }
                        } else {
                            return Err(syn::Error::new(
                                value.span(),
                                "file_name must be a literal",
                            ));
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

        let file_name = file_name.ok_or_else(|| {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "missing required attribute: #[config(file_name = \"...\")]",
            )
        })?;

        Ok(Self { version, file_name })
    }
}

/// Derive macro for the `Config` trait.
///
/// This macro automatically:
/// - Implements the `Config` trait with the specified version and file name
/// - Registers the config type with `inventory`
///
///
/// # Example
///
/// ```rust
/// use next_config::Config;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Debug, Default, Serialize, Deserialize, Config)]
/// #[config(version = 1, file_name = "app.toml")]
/// struct AppConfig {
///     name: String,
///     port: u16,
/// }
/// ```
///
/// This expands to roughly:
///
/// ```rust
/// // ... your struct with serde derives ...
///
/// impl next_config::Config for AppConfig {
///     const VERSION: u32 = 1;
///     const FILE_NAME: &'static str = "app.toml";
/// }
///
/// inventory::submit! {
///     next_config::RegisteredConfig::new::<AppConfig>()
/// }
/// ```
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
    let file_name = &options.file_name;

    // Generate the Config trait implementation
    let config_impl = quote! {
        impl ::next_config::Config for #name {
            const VERSION: u32 = #version;
            const FILE_NAME: &'static str = #file_name;
        }
    };

    Ok(config_impl)
}
