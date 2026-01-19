use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, LitStr, Pat};

#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro(attr, item)
}

#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro(attr, item)
}

#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro(attr, item)
}

#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro(attr, item)
}

fn route_macro_core(attr: proc_macro2::TokenStream, item: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let _path: LitStr = syn::parse2(attr).expect("expected path as string literal");
    let func: ItemFn = syn::parse2(item).expect("expected function");

    let func_name = &func.sig.ident;
    let func_block = &func.block;
    let func_output = &func.sig.output;
    let func_vis = &func.vis;

    let args: Vec<_> = func.sig.inputs.iter().collect();

    let expanded = if args.is_empty() {
        quote! {
            #func_vis async fn #func_name(
                _req: hyper::Request<hyper::body::Incoming>,
                _params: rapina::extract::PathParams,
            ) #func_output #func_block
        }
    } else {
        let mut extractions = Vec::new();
        let mut arg_names = Vec::new();

        for arg in &args {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    let arg_name = &pat_ident.ident;
                    let arg_type = &pat_type.ty;

                    arg_names.push(arg_name.clone());
                    extractions.push(quote! {
                        let #arg_name = <#arg_type as rapina::extract::FromRequest>::from_request(req, &params).await.unwrap();
                    });
                }
            }
        }

        let inner_block = &func.block;

        quote! {
            #func_vis async fn #func_name(
                req: hyper::Request<hyper::body::Incoming>,
                params: rapina::extract::PathParams,
            ) #func_output {
                #(#extractions)*
                #inner_block
            }
        }
    };

    expanded
}

fn route_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro_core(attr.into(), item.into()).into()
}

#[cfg(test)]
mod tests {
    use  super::route_macro_core;
    use proc_macro2::TokenStream;
    use quote::quote;
    use syn::ItemFn;

    #[test]
    fn test_function_with_no_args() {
        let path = quote!("/");
        let input = quote! {
            async fn hello() -> &'static str {
                "Hello, Rapina!"
            }
        };

        let output = route_macro_core(path, input);

        // parse back and verify structure
        let output_fn: ItemFn = syn::parse2(output).expect("expected function");

        // check should have 2 parameters: req and params
        assert_eq!(output_fn.sig.inputs.len(), 2);

        // check if function name preserved
        assert_eq!(output_fn.sig.ident.to_string(), "hello");
    }

    #[test]
fn test_function_with_single_extractor() {
    let path = quote!("/users/:id");
    let input = quote! {
        async fn get_user(id: rapina::extract::Path<u64>) -> String {
            format!("{}", id.into_inner())
        }
    };
    
    let output = route_macro_core(path, input);
    let output_func: ItemFn = syn::parse2(output).expect("should parse as function");
    
    // check should have 2 parameters: req and params
    assert_eq!(output_func.sig.inputs.len(), 2);
    
    let body_str = quote!(#output_func.block).to_string();

    // check should contain extraction code
    assert!(body_str.contains("from_request"));
    assert!(body_str.contains("id"));
}

#[test]
fn test_function_with_multiple_extractors() {
    let path = quote!("/users");
    let input = quote! {
        async fn create_user(
            id: rapina::extract::Path<u64>,
            body: rapina::extract::Json<String>
        ) -> String {
            "created".to_string()
        }
    };
    
    let output = route_macro_core(path, input);
    let output_func: ItemFn = syn::parse2(output).expect("should parse as function");
    
    // Check signature has req and params
    assert_eq!(output_func.sig.inputs.len(), 2);
    
    // Check both extractions are present in body
    let body_str = quote!(#output_func.block).to_string();
    assert!(body_str.contains("id"));
    assert!(body_str.contains("body"));
}

#[test]
#[should_panic(expected = "expected function")]
fn test_invalid_input_panics() {
    let path = quote!("/");
    let invalid_input = quote! { not_a_function };
    
    route_macro_core(path, invalid_input);
}
    
}