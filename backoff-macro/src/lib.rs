#![feature(proc_macro)]

extern crate futures;
extern crate proc_macro;
extern crate proc_macro2;

#[macro_use]
extern crate syn;

#[macro_use]
extern crate quote;
extern crate darling;

use darling::FromMeta;
use proc_macro::TokenStream;

#[derive(Default, Debug, FromMeta)]
#[darling(default)]
struct OnErrorOptions {
    max_tries: Option<u64>,
    max_time: Option<u64>,
}

use quote::{ToTokens, TokenStreamExt};
use std::ops::Deref;
use syn::spanned::Spanned;

struct QuoteOption<T>(Option<T>);

impl<T> ToTokens for QuoteOption<T>
where
    T: ToTokens,
{
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.append_all(match self.0 {
            Some(ref t) => quote! { ::std::option::Option::Some(#t) },
            None => quote! { ::std::option::Option::None },
        });
    }
}

fn parse_meta(tokens: TokenStream) -> ::std::result::Result<syn::Meta, String> {
    let tokens: proc_macro2::TokenStream = tokens.into();
    let attribute: syn::Attribute = parse_quote!(#[dummy(#tokens)]);
    attribute.interpret_meta().ok_or("Unable to parse".into())
}

fn on_error_impl(mut func: syn::ItemFn, options: OnErrorOptions) -> syn::ItemFn {
    let max_time = QuoteOption(options.max_time);
    let max_tries = QuoteOption(options.max_tries);
    let closure = make_closure_from_fn(&func);
    let check = check_fn(&func);
    let body: syn::Block = parse_quote!({
        #check
        let backoff = ::backoff::Backoff::new(#max_tries, #max_time, #closure);
        backoff
    });
    {
        let ty = extract_return_type(&func);
        func.decl.output = parse_quote!(-> ::backoff::Backoff<#ty>);
    }
    let wrapped = syn::ItemFn {
        block: Box::new(body),
        ..func
    };
    wrapped
}

fn make_closure_from_fn(func: &syn::ItemFn) -> proc_macro2::TokenStream {
    let block = &func.block;
    let closure = quote!(move || #block);
    closure
}

fn check_fn(func: &syn::ItemFn) -> proc_macro2::TokenStream {
    let ty = &func.decl.output;
    match ty {
        &syn::ReturnType::Default => {
            ty.span()
                .unstable()
                .error("Only functions returning Future is allowed")
                .emit();
            proc_macro2::TokenStream::new()
        }
        &syn::ReturnType::Type(_, ref ret_ty) => {
            let ty_span = ret_ty.span();

            if let syn::Type::ImplTrait(_) = ret_ty.deref() {
                ret_ty
                    .span()
                    .unstable()
                    .error("functions returning impl Trait not allowed")
                    .emit();
            }
            let assert_future = quote_spanned! {ty_span=>
                struct _AssertFuture where #ret_ty: ::backoff::Future;
            };
            assert_future
        }
    }
}

fn extract_return_type(func: &syn::ItemFn) -> Box<syn::Type> {
    let ty = &func.decl.output;
    match ty {
        &syn::ReturnType::Default => unreachable!("Default type reached"),
        &syn::ReturnType::Type(_, ref ty) => ty.clone(),
    }
}

#[proc_macro_attribute]
pub fn on_error(args: TokenStream, input: TokenStream) -> TokenStream {
    let meta = parse_meta(args).unwrap();
    let opts = OnErrorOptions::from_meta(&meta).unwrap();

    let func: syn::ItemFn = match syn::parse(input.clone()) {
        Ok(input) => input,
        Err(_) => {
            return input;
        }
    };
    on_error_impl(func, opts).into_token_stream().into()
}
