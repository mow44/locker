use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(ImplHelper, attributes(helper))]
pub fn impl_helper(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let mut methods = Vec::default();

    if let Data::Struct(data_struct) = input.data {
        if let Fields::Named(fields) = data_struct.fields {
            for field in fields.named.iter() {
                let field_name = field.ident.as_ref().unwrap();
                let field_type = &field.ty;

                let mut make_get = false;
                let mut make_get_mut = false;
                let mut make_set = false;
                let mut make_upd = false;

                for attr in &field.attrs {
                    if attr.path().is_ident("helper") {
                        if let Err(e) = attr.parse_nested_meta(|meta| {
                            if meta.path.is_ident("get") {
                                make_get = true;
                                return Ok(());
                            }

                            if meta.path.is_ident("get_mut") {
                                make_get_mut = true;
                                return Ok(());
                            }

                            if meta.path.is_ident("set") {
                                make_set = true;
                                return Ok(());
                            }

                            if meta.path.is_ident("upd") {
                                make_upd = true;
                                return Ok(());
                            }

                            if meta.path.is_ident("all") {
                                make_get = true;
                                make_get_mut = true;
                                make_set = true;
                                make_upd = true;
                                return Ok(());
                            }

                            Err(meta.error(format!(
                                "Unexpected expression in macro: {:#?}.
                                Expected one of: get, get_mut, set, upd or all",
                                meta.path
                                    .segments
                                    .iter()
                                    .map(|segment| segment.ident.to_string())
                                    .collect::<Vec<_>>()
                                    .join("::")
                            )))
                        }) {
                            return syn::Error::new_spanned(attr, format!("{}", e))
                                .to_compile_error()
                                .into();
                        }
                    }
                }

                if make_get {
                    methods.push(quote! {
                        pub fn #field_name(&self) -> &#field_type {
                            &self.#field_name
                        }
                    });
                }

                if make_get_mut {
                    let fn_name =
                        syn::Ident::new(&format!("{}_mut", field_name), field_name.span());

                    methods.push(quote! {
                        pub fn #fn_name(&mut self) -> &mut #field_type {
                            &mut self.#field_name
                        }
                    });
                }

                if make_set {
                    let fn_name =
                        syn::Ident::new(&format!("{}_set", field_name), field_name.span());

                    methods.push(quote! {
                        #[must_use = "method moves the value of self and returns the modified value"]
                        pub fn #fn_name(mut self, value: #field_type) -> Self {
                            self.#field_name = value;
                            self
                        }
                    });
                }

                if make_upd {
                    let fn_name =
                        syn::Ident::new(&format!("{}_update", field_name), field_name.span());

                    methods.push(quote! {
                        pub fn #fn_name(&mut self, value: #field_type) -> &mut Self {
                            self.#field_name = value;
                            self
                        }
                    });
                }
            }
        }
    }

    TokenStream::from(quote! {
        impl #impl_generics #name #type_generics #where_clause {
            #(#methods)*
        }
    })
}
