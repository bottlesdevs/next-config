use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Ident, ItemStruct, LitInt, LitStr, Token,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

/// Parsed arguments of #[config(...)]
struct ConfigArgs {
    name: Option<LitStr>,
    version: Option<LitInt>,
}

impl Parse for ConfigArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut version = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            input.parse::<Token![=]>()?;

            if ident == "name" {
                name = Some(input.parse()?);
            } else if ident == "version" {
                version = Some(input.parse()?);
            } else {
                return Err(syn::Error::new(
                    ident.span(),
                    "expected `name` or `version`",
                ));
            }

            // Consume optional trailing comma
            let _ = input.parse::<Token![,]>();
        }

        Ok(Self { name, version })
    }
}

#[proc_macro_attribute]
pub fn config(args: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(args as ConfigArgs);
    let input = parse_macro_input!(item as ItemStruct);

    let ident = &input.ident;

    let file_name = args
        .name
        .map(|s| s.value())
        .unwrap_or_else(|| ident.to_string().to_lowercase());

    let file_name = format!("{file_name}.toml");

    let version = args
        .version
        .map(|v| v.base10_parse::<u32>().unwrap_or(1))
        .unwrap_or(1);

    quote! {
        #[derive(serde::Serialize, serde::Deserialize)]
        #input

        impl ::next_config::Config for #ident {
            const FILE_NAME: &'static str = #file_name;
            const VERSION: u32 = #version;
        }
    }
    .into()
}
