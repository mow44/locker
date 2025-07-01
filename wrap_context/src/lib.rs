use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, Expr,
    Member::{Named, Unnamed},
};

/// Use this instead of `anyhow::anyhow!()`
#[proc_macro]
pub fn wohyna(input: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(input);

    let output = quote! {
        {
            use anyhow::anyhow;

            let loc = std::panic::Location::caller();

            anyhow!(
                "[{}:{}:{}] {}",
                loc.file(),
                loc.line(),
                loc.column(),
                format!(#args)
            )
        }
    };

    output.into()
}

/// Use this instead of `anyhow::bail!()`
#[proc_macro]
pub fn liab(input: TokenStream) -> TokenStream {
    let args = proc_macro2::TokenStream::from(input);

    let output = quote! {
        {
            use anyhow::bail;

            let loc = std::panic::Location::caller();

            bail!(
                "[{}:{}:{}] {}",
                loc.file(),
                loc.line(),
                loc.column(),
                format!(#args)
            )
        }
    };

    output.into()
}

#[proc_macro]
pub fn arg_context(input: TokenStream) -> TokenStream {
    let expr = parse_macro_input!(input as Expr);

    let result = describe_expr(&expr, true);

    let output = quote! {
        {
            use anyhow::Context;

            let loc = std::panic::Location::caller();

            #expr.with_context(|| {
                format!(
                    "[{}:{}:{}] {}",
                    loc.file(),
                    loc.line(),
                    loc.column(),
                    #result
                )
            })
        }
    };

    output.into()
}

#[proc_macro]
pub fn raw_context(input: TokenStream) -> TokenStream {
    let expr = parse_macro_input!(input as Expr);

    let result = describe_expr(&expr, false);

    let output = quote! {
        {
            use anyhow::Context;

            let loc = std::panic::Location::caller();

            #expr.with_context(|| {
                format!(
                    "[{}:{}:{}] {}",
                    loc.file(),
                    loc.line(),
                    loc.column(),
                    #result
                )
            })
        }
    };

    output.into()
}

fn describe_expr(expr: &Expr, print_args: bool) -> proc_macro2::TokenStream {
    match expr {
        Expr::Call(call) => {
            let func_name = describe_expr(&call.func, print_args);

            if print_args {
                let arg_to_string_iter = call.args.iter().map(|arg| {
                    quote!(crate::utils::clip_string(
                        format!("{:?}", #arg),
                        "…",
                        *crate::types::DEBUG_PRINT_LIMIT.get().unwrap_or(&1) // FIXME move to utils
                    ))
                });

                quote! {
                    {
                        let args_list = <Vec<String>>::from([#(#arg_to_string_iter),*]).join(", ");
                        format!("{}({})", #func_name, args_list)
                    }
                }
            } else {
                quote! {
                    {
                        format!("{}(?)", #func_name)
                    }
                }
            }
        }
        Expr::MethodCall(method) => {
            let receiver_name = describe_expr(&method.receiver, print_args);
            let method_name = method.method.to_string();

            if print_args {
                let arg_to_string_iter = method.args.iter().map(|arg| {
                    quote!(crate::utils::clip_string(
                        format!("{:?}", #arg),
                        "…",
                        *crate::types::DEBUG_PRINT_LIMIT.get().unwrap_or(&1)
                    ))
                });

                quote! {
                    {
                        let args_list = <Vec<String>>::from([#(#arg_to_string_iter),*]).join(", ");
                        format!(
                            "{}.{}({})",
                            #receiver_name,
                            #method_name,
                            args_list
                        )
                    }
                }
            } else {
                quote! {
                    {
                        format!(
                            "{}.{}(?)",
                            #receiver_name,
                            #method_name
                        )
                    }
                }
            }
        }
        Expr::Path(path) => {
            let path = path
                .to_token_stream()
                .to_string()
                .replace(" :: ", "::")
                .replace(" . ", ".");
            quote!(format!("{}", #path))
        }
        Expr::Field(field) => {
            let base_name = describe_expr(&field.base, print_args);
            let member_name = match &field.member {
                Named(ident) => ident.to_token_stream().to_string(),
                Unnamed(index) => index.index.to_string(),
            };

            quote!(format!("{}.{}", #base_name, #member_name))
        }
        Expr::Await(aw) => {
            let base_name = describe_expr(&aw.base, print_args);

            quote!(format!("{}.await", #base_name))
        }
        Expr::Macro(ma) => {
            let path = ma
                .mac
                .path
                .to_token_stream()
                .to_string()
                .replace(" :: ", "::")
                .replace(" . ", ".");
            let tokens = ma.mac.tokens.to_token_stream().to_string();

            quote! {
                {
                    format!(
                        "{}!({})",
                        #path,
                        #tokens
                    )
                }
            }
        }
        Expr::Try(tr) => {
            let inner = describe_expr(&tr.expr, print_args);

            quote! {
                {
                    format!(
                        "{}",
                        #inner
                    )
                }
            }
        }
        _ => syn::Error::new_spanned(expr, format!("Unexpected expression in macro: {:#?}", expr))
            .to_compile_error()
            .into(),
    }
}
