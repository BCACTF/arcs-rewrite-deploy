use std::str::FromStr;

use proc_macro::TokenStream;
use quote::{quote, __private::TokenStream as QuoteTokenStream};
use syn::Ident;


fn parse_into_ident(ident: &str) -> Result<Ident, QuoteTokenStream> {
    syn::parse::<Ident>(
        TokenStream
            ::from_str(ident)
            .map_err(|error| ToString::to_string(&error))
            .map_err(|error| quote!{
                compile_error!(#error);
            })?,
    ).map_err(
        |err| err.to_compile_error(),
    )
}

fn parse_into_idents(idents: (&str, &str, &str)) -> Result<(Ident, Ident, Ident), QuoteTokenStream> {
    match (parse_into_ident(idents.0), parse_into_ident(idents.1), parse_into_ident(idents.2)) {
        (Ok(ident_1), Ok(ident_2), Ok(ident_3)) => Ok((ident_1, ident_2, ident_3)),
        (Err(e_1), Err(e_2), Err(e_3)) => Err(quote!{ #e_1 #e_2 #e_3 }),
        (Err(e_1), Err(e_2), Ok(_)) => Err(quote!{ #e_1 #e_2 }),
        (Err(e_1), Ok(_), Err(e_3)) => Err(quote!{ #e_1 #e_3 }),
        (Err(e_1), Ok(_), Ok(_)) => Err(quote!{ #e_1 }),
        (Ok(_), Err(e_2), Err(e_3)) => Err(quote!{ #e_2 #e_3 }),
        (Ok(_), Err(e_2), Ok(_)) => Err(quote!{ #e_2 }),
        (Ok(_), Ok(_), Err(e_3)) => Err(quote!{ #e_3 }),
    }
}

#[proc_macro]
pub fn with_target(input: TokenStream) -> TokenStream {
    let lit: Result<syn::LitStr, _> = syn::parse(input);

    match lit {
        Ok(lit) => {
            let macros = [
                ("error", "error", "log_error"),
                ("warn", "warn", "log_warn"),
                ("info", "info", "log_info"),
                ("debug", "debug", "log_debug"),
                ("trace", "trace", "log_trace"),
            ];
            let inner_stream: QuoteTokenStream = macros
                .into_iter()
                .map(parse_into_idents)
                .map(|name| match name {
                    Ok(idents) => {
                        let (orig_macro_name, export_name, this_macro_name) = idents;
                        quote! {
                            macro_rules! #this_macro_name {
                                (target: $target:expr, $($arg:tt)+) => {
                                    arcs_deploy_logging::__internal_redirects::#orig_macro_name!(target: $target, $($arg)+)
                                };
                                ($($arg:tt)+) => {
                                    arcs_deploy_logging::__internal_redirects::#orig_macro_name!(target: #lit, $($arg)+)
                                };
                            }
                            pub(crate) use #this_macro_name as #export_name;
                        }
                    },
                    Err(err) => err
                })
                .collect();

            
            let macro_names = [
                "error",
                "warn",
                "info",
                "debug",
                "trace",
            ];
            let use_stream: QuoteTokenStream = macro_names
                .into_iter()
                .map(parse_into_ident)
                .map(|result| match result {
                    Ok(ident) => quote!{ #ident, },
                    Err(err) => err
                })
                .collect();

            quote!{
                #[doc(hidden)]
                mod __internal_logging_macros {
                    #inner_stream
                }

                pub(crate) use __internal_logging_macros::{#use_stream};

                pub static DEFAULT_TARGET_NAME: &str = #lit;

            }.into()
        },
        Err(err) => {
            let compile_error_tokens = err.to_compile_error();
            quote!{ 
                #compile_error_tokens
            }
        }.into()
    }
}