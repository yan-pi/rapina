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

fn route_macro_core(
    attr: proc_macro2::TokenStream,
    item: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let _path: LitStr = syn::parse2(attr).expect("expected path as string literal");
    let func: ItemFn = syn::parse2(item).expect("expected function");

    let func_name = &func.sig.ident;
    let func_block = &func.block;
    let func_output = &func.sig.output;
    let func_vis = &func.vis;

    let args: Vec<_> = func.sig.inputs.iter().collect();

    if args.is_empty() {
        quote! {
            #func_vis async fn #func_name(
                req: hyper::Request<hyper::body::Incoming>,
                params: rapina::extract::PathParams,
                state: std::sync::Arc<rapina::state::AppState>,
            ) #func_output #func_block
        }
    } else {
        let mut parts_extractions = Vec::new();
        let mut body_extractors: Vec<(syn::Ident, Box<syn::Type>)> = Vec::new();
        let mut arg_names = Vec::new();

        for arg in &args {
            if let FnArg::Typed(pat_type) = arg
                && let Pat::Ident(pat_ident) = &*pat_type.pat
            {
                let arg_name = &pat_ident.ident;
                let arg_type = &pat_type.ty;

                arg_names.push(arg_name.clone());

                // Check if this is a parts-only extractor
                let type_str = quote!(#arg_type).to_string();
                if is_parts_only_extractor(&type_str) {
                    parts_extractions.push(quote! {
                        let #arg_name = match <#arg_type as rapina::extract::FromRequestParts>::from_request_parts(&parts, &params, &state).await {
                            Ok(v) => v,
                            Err(e) => return rapina::response::IntoResponse::into_response(e),
                        };
                    });
                } else {
                    body_extractors.push((arg_name.clone(), arg_type.clone()));
                }
            }
        }

        // Generate body extraction code - only one body extractor is allowed
        let body_extraction = if body_extractors.is_empty() {
            quote! {}
        } else if body_extractors.len() == 1 {
            let (arg_name, arg_type) = &body_extractors[0];
            quote! {
                let req = http::Request::from_parts(parts, body);
                let #arg_name = match <#arg_type as rapina::extract::FromRequest>::from_request(req, &params, &state).await {
                    Ok(v) => v,
                    Err(e) => return rapina::response::IntoResponse::into_response(e),
                };
            }
        } else {
            let names: Vec<_> = body_extractors.iter().map(|(n, _)| n.to_string()).collect();
            panic!(
                "Multiple body-consuming extractors are not supported: {}. Only one extractor can consume the request body.",
                names.join(", ")
            );
        };

        let inner_block = &func.block;

        quote! {
            #func_vis async fn #func_name(
                req: hyper::Request<hyper::body::Incoming>,
                params: rapina::extract::PathParams,
                state: std::sync::Arc<rapina::state::AppState>,
            ) -> hyper::Response<http_body_util::Full<hyper::body::Bytes>> {
                let (parts, body) = req.into_parts();
                #(#parts_extractions)*
                #body_extraction
                rapina::response::IntoResponse::into_response((|| async #inner_block)().await)
            }
        }
    }
}

fn is_parts_only_extractor(type_str: &str) -> bool {
    type_str.contains("Path")
        || type_str.contains("Query")
        || type_str.contains("State")
        || type_str.contains("Context")
}

fn route_macro(attr: TokenStream, item: TokenStream) -> TokenStream {
    route_macro_core(attr.into(), item.into()).into()
}

#[cfg(test)]
mod tests {
    use super::route_macro_core;

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

        // check should have 3 parameters: req, params, and state
        assert_eq!(output_fn.sig.inputs.len(), 3);

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

        // check should have 3 parameters: req, params, and state
        assert_eq!(output_func.sig.inputs.len(), 3);

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

        // Check signature has req, params, and state
        assert_eq!(output_func.sig.inputs.len(), 3);

        // Check both extractions are present in body
        let body_str = quote!(#output_func.block).to_string();
        assert!(body_str.contains("id"));
        assert!(body_str.contains("body"));

        // Check that parts-only extractor uses FromRequestParts
        assert!(body_str.contains("FromRequestParts"));
        // Check that body extractor uses FromRequest
        assert!(body_str.contains("FromRequest"));
    }

    #[test]
    fn test_parts_extractors_before_body() {
        let path = quote!("/users/:id");
        let input = quote! {
            async fn handler(
                ctx: rapina::extract::Context,
                id: rapina::extract::Path<u64>,
                state: rapina::extract::State<Config>,
                body: rapina::extract::Json<Data>
            ) -> String {
                "ok".to_string()
            }
        };

        let output = route_macro_core(path, input);
        let body_str = output.to_string();

        // Verify request is split into parts and body
        assert!(body_str.contains("into_parts"));

        // Verify parts extractors use FromRequestParts
        assert!(body_str.contains("FromRequestParts"));

        // Verify body extractor reconstructs request
        assert!(body_str.contains("from_parts"));
    }

    #[test]
    #[should_panic(expected = "Multiple body-consuming extractors are not supported")]
    fn test_multiple_body_extractors_panics() {
        let path = quote!("/users");
        let input = quote! {
            async fn handler(
                body1: rapina::extract::Json<String>,
                body2: rapina::extract::Json<String>
            ) -> String {
                "ok".to_string()
            }
        };

        route_macro_core(path, input);
    }

    #[test]
    #[should_panic(expected = "expected function")]
    fn test_invalid_input_panics() {
        let path = quote!("/");
        let invalid_input = quote! { not_a_function };

        route_macro_core(path, invalid_input);
    }
}
