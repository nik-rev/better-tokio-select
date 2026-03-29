#![doc = concat!("[![crates.io](https://img.shields.io/crates/v/", env!("CARGO_PKG_NAME"), "?style=flat-square&logo=rust)](https://crates.io/crates/", env!("CARGO_PKG_NAME"), ")")]
#![doc = concat!("[![docs.rs](https://img.shields.io/docsrs/", env!("CARGO_PKG_NAME"), "?style=flat-square&logo=docs.rs)](https://docs.rs/", env!("CARGO_PKG_NAME"), ")")]
#![doc = "![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)"]
#![doc = concat!("![msrv](https://img.shields.io/badge/msrv-", "nightly", "-blue?style=flat-square&logo=rust)")]
//! [![github](https://img.shields.io/github/stars/nik-rev/better-tokio-select)](https://github.com/nik-rev/better-tokio-select)
//!
//! This crate exports the macro [`#[tokio_select]`](tokio_select), which, unlike [`tokio::select!`](https://docs.rs/tokio/latest/tokio/macro.select.html), can be formatted by `rustfmt`!
//!
//! ```toml
#![doc = concat!(env!("CARGO_PKG_NAME"), " = ", "\"", env!("CARGO_PKG_VERSION_MAJOR"), ".", env!("CARGO_PKG_VERSION_MINOR"), "\"")]
//! ```
//!
//! # Syntax
//!
//! This macro has all the same capabilities as `tokio::select!`, but the syntax is *slightly* different.
//!
//! `tokio::select!` takes a list of branches:
//!
//! ```txt
//! <pattern> = <async expression> (, if <precondition>)? => <handler>,
//! ```
//!
//! Example:
//!
//! ```
//! # /*
//! tokio::select! {
//!     Ok(res) = reader.read(&mut buf), if can_read => {
//!         writer.write_all(res.bytes)
//!     }
//! }
//! # */
//! ```
//!
//! `#[tokio_select]` applies to a `match` expression, which has a list of arms:
//!
//! ```txt
//! .. if let <pattern> = <async expression> (&& <precondition>)? => <handler>,
//! ```
//!
//! Example:
//!
//! ```
//! # /*
//! tokio_select!(match .. {
//!     .. if let Ok(res) = reader.read(&mut buf) && can_read => {
//!         writer.write_all(res.bytes)
//!     }
//! })
//! # */
//! ```
//!
//! # Examples
//!
//! ## TCP Proxy with Cancellation and Guard
//!
//! `tokio::select!`:
//!
//! ```
//! # /*
//! tokio::select! {
//!     res = reader.read(&mut buf), if can_read => {
//!         let n = res?;
//!         if n == 0 { return Ok(()); }
//!         writer.write_all(&buf[..n]).await?;
//!     }
//!
//!     _ = shutdown.recv() => {
//!         return Ok(());
//!     }
//! }
//! # */
//! ```
//!
//! `#[tokio_select]`:
//!
//! ```
//! # /*
//! tokio_select!(match .. {
//!     .. if let Ok(n) = reader.read(&mut buf) && can_read => {
//!         let n = res?;
//!         if n == 0 { return Ok(()); }
//!         writer.write_all(&buf[..n]).await?;
//!     }
//!
//!     .. if let _ = shutdown.recv() => {
//!         return Ok(())
//!     }
//! })
//! # */
//! ```
//!
//! Admittedly, the syntax is a little strange. But it's also formattable by `rustfmt`. Trade-offs, people, trade-offs!
//!
//! ## Rate-Limited Message Processor
//!
//! ```
//! # /*
//! tokio::select! {
//!     biased;
//!
//!     Some(Message::Data { id, payload }) = rx.recv() => {
//!         process(id, payload).await;
//!     }
//!
//!     else => {
//!         println!("no messages pending");
//!         tokio::time::sleep(Duration::from_millis(50)).await;
//!     }
//! }
//! # */
//! ```
//!
//! `#[tokio_select]`:
//!
//! ```
//! # /*
//! tokio_select!(biased, match .. {
//!     .. if let Some(Message::Data { id, payload }) = rx.recv() => {
//!         process(id, payload).await;
//!     }
//!
//!     _ => {
//!         println!("no messages pending");
//!         tokio::time::sleep(Duration::from_millis(50)).await;
//!     }
//! })
//! # */
//! ```
//!
//! # Global import
//!
//! You can make the `tokio_select!` macro globally available in your crate, without needing to import it, with:
//!
//! ```
//! #[macro_use(tokio_select)]
//! extern crate better_tokio_select;
//! ```
#![allow(rustdoc::invalid_rust_codeblocks)]

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenTree;
use quote::quote;
use quote::ToTokens;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;
use syn::spanned::Spanned;
use syn::Arm;
use syn::Expr;
use syn::ExprLet;
use syn::ExprMatch;
use syn::Pat;
use syn::PatRest;
use syn::RangeLimits;
use syn::Token;

/// Like `tokio::select!`, but formattable by `rustfmt`.
///
/// This macro has all the same capabilities as [`tokio::select!`], but the syntax is *slightly* different.
///
/// [`tokio::select!`]: https://docs.rs/tokio/latest/tokio/macro.select.html
///
/// `tokio::select!` takes a list of branches:
///
/// ```txt
/// <pattern> = <async expression> (, if <precondition>)? => <handler>,
/// ```
///
/// Example:
///
/// ```
/// # /*
/// tokio::select! {
///     Ok(res) = reader.read(&mut buf), if can_read => {
///         writer.write_all(res.bytes)
///     }
/// }
/// # */
/// ```
///
/// `#[tokio_select]` applies to a `match` expression, which has a list of arms:
///
/// ```txt
/// () if let <pattern> = <async expression> (&& <precondition>)? => <handler>,
/// ```
///
/// Example:
///
/// ```
/// # /*
/// #[tokio_select]
/// match () {
///     () if let Ok(res) = reader.read(&mut buf) && can_read => {
///         writer.write_all(res.bytes)
///     }
/// }
/// # */
/// ```
///
/// See the [crate-level](crate) documentation for more info.
#[proc_macro]
pub fn tokio_select(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as SelectInput);
    let is_biased = input.is_biased;
    let match_expr = input.match_expr;

    let mut select_arms = if is_biased {
        quote! { biased; }
    } else {
        quote! {}
    };

    let mut errors: Vec<(String, Span)> = Vec::new();

    // no #[attr]s allowed on the `match` expression
    if let Some(attr) = match_expr.attrs.first() {
        errors.push(("no attributes expected".into(), attr.span()));
    }

    // scrutinee must be exactly `()`
    match &*match_expr.expr {
        Expr::Range(syn::ExprRange {
            attrs,
            start: None,
            limits: RangeLimits::HalfOpen(_),
            end: None,
        }) => {
            // no #[attr]s allowed on scrutinee
            if let Some(attr) = attrs.first() {
                errors.push(("no attributes expected".into(), attr.span()));
            }
        }
        _ => {
            errors.push(("expected `..`".into(), match_expr.expr.span()));
        }
    }

    for Arm {
        pat, guard, body, ..
    } in match_expr.arms
    {
        // pattern must be exactly `..`
        match &pat {
            // .. if let () = std::future::ready(()) => {}
            // ^^
            Pat::Rest(PatRest { attrs, .. }) => {
                // no #[attr]s allowed on pattern
                if let Some(attr) = attrs.first() {
                    errors.push(("no attributes expected".into(), attr.span()));
                }
            }
            // when we encounter a `_ => {...}` arm, treat that specially
            // (no `if let` guard is required)
            //
            // _ => break
            // ^
            Pat::Wild(_) => {
                // in the `else =>` branch, no condition is allowed
                if let Some((kw, _)) = guard {
                    errors.push(("no guard expected".into(), kw.span()));
                }

                select_arms.extend(quote! {
                    else => #body
                });

                continue;
            }
            // any other pattern is an error
            _ => {
                errors.push(("expected `..`".into(), pat.span()));
            }
        }

        // if let foo = bar && baz && quux
        // ^^ _guard_kw_if
        //    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^ guard
        let Some((_guard_kw_if, guard)) = guard else {
            errors.push(("expected `if let` guard after pattern".into(), pat.span()));
            continue;
        };

        let mut tokens = guard.into_token_stream().into_iter().peekable();

        // if let foo = bar && baz && quux
        //    ^^^^^^^^^^^^^
        let mut expr_let_tokens = proc_macro2::TokenStream::new();

        // The way that `syn` parses an `if let` expression is like so:
        //
        // if let foo = bar && baz && quux
        //    ^^^^^^^^^^^^^^^^^^^^^--^^^^^ Expr::Binary(ExprBinary { op: OpAnd })
        //    ^^^^^^^^^^^^^^--^^^^ Expr::Binary(ExprBinary { op: OpAnd })
        //    ^^^^^^^^^^^^^ Expr::Let(ExprLet)
        //
        // But what we really want is to extract everything after the first `&&` into
        // its own conditioin:
        //
        // if let foo = bar && baz && quux
        //                    ^^^^^^^^^^^ we want this to be <condition>
        //        ^^^ the <pat>
        //            ^^^ the <async expr>
        //
        // So we must manually parse a flat list of tokens here, until we find '&&'
        let has_if_guard = loop {
            match (tokens.next(), tokens.peek()) {
                (Some(TokenTree::Punct(p)), Some(TokenTree::Punct(p2)))
                    if p.as_char() == '&' && p2.as_char() == '&' =>
                {
                    let _ = tokens.next();
                    break true;
                }
                (None, _) => {
                    break false;
                }
                (Some(tt), _) => {
                    expr_let_tokens.extend([tt]);
                    continue;
                }
            }
        };

        // if let foo = bar && baz && quux
        //                    ^^^^^^^^^^^
        let if_guard = has_if_guard.then(|| tokens.collect::<proc_macro2::TokenStream>());

        // if let foo = bar && baz && quux
        //    ^^^^^^^^^^^^^
        let expr_let = match syn::parse2::<ExprLet>(expr_let_tokens) {
            Ok(expr_let) => expr_let,
            Err(err) => {
                return err.into_compile_error().into();
            }
        };

        if let Some(attr) = expr_let.attrs.first() {
            errors.push(("no attributes expected".into(), attr.span()));
            continue;
        }

        let pat = &*expr_let.pat;
        let async_expr = &*expr_let.expr;
        let if_guard = if_guard.into_iter();

        // Generate a single branch of `tokio::select!`

        select_arms.extend(quote! {
            #pat = #async_expr #(, if #if_guard)* => #body,
        });
    }

    if !errors.is_empty() {
        let compile_errors = errors.into_iter().map(|(msg, span)| {
            quote::quote_spanned! { span => compile_error!(#msg); }
        });
        return quote! { #(#compile_errors)* }.into();
    }

    quote! {
        ::tokio::select! {
            #select_arms
        }
    }
    .into()
}

struct SelectInput {
    is_biased: bool,
    match_expr: ExprMatch,
}

impl Parse for SelectInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let is_biased = if input.peek(syn::Ident) && input.peek2(Token![,]) {
            let id: syn::Ident = input.parse()?;
            if id == "biased" {
                let _: Token![,] = input.parse()?;
                true
            } else {
                return Err(syn::Error::new(id.span(), "expected `biased`"));
            }
        } else {
            false
        };

        let match_expr: ExprMatch = input.parse()?;
        Ok(SelectInput {
            is_biased,
            match_expr,
        })
    }
}

mod kw {
    syn::custom_keyword!(biased);
}
