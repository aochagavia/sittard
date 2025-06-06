use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{ItemFn, parse_macro_input};

fn test_impl(
    runtime_path: proc_macro2::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);

    // Reject non-async functions
    let asyncness = input_fn.sig.asyncness.is_some();
    if !asyncness {
        // Link the error to the function signature
        let sig_span = input_fn.sig.span();
        let error = quote_spanned! {sig_span=>
            compile_error!("this attribute can only be used on async functions");
        };
        return error.into();
    }

    // Generate the new function
    let fn_name = &input_fn.sig.ident;
    let fn_block = &input_fn.block;
    let vis = &input_fn.vis;
    let attrs = &input_fn.attrs;
    let output = quote! {
        #(#attrs)*
        #[test]
        #vis fn #fn_name() {
            // Initialize runtime and block on the async function
            let runtime = #runtime_path;
            runtime.block_on(async #fn_block)
        }
    };

    output.into()
}

#[proc_macro_attribute]
pub fn test_priv(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let runtime_path = quote! { crate::Runtime::default() };
    test_impl(runtime_path, item)
}

#[proc_macro_attribute]
pub fn test(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let runtime_path = quote! { sittard::Runtime::default() };
    test_impl(runtime_path, item)
}
