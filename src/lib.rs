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
//!     Ok(res) = reader.read(&mut buf), if can_read => writer.write_all(res.bytes)
//! }
//! # */
//! ```
//!
//! `#[tokio_select]` applies to a `match` expression, which has a list of arms:
//!
//! ```txt
//! <pattern> | on!(<async expression>) (if <precondition>)? => <handler>,
//! ```
//!
//! Example:
//!
//! ```
//! # /*
//! match () {
//!     Ok(res) | on!(reader.read(&mut buf)) if can_read => writer.write_all(res.bytes)
//! }
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
//! #[tokio_select]
//! match () {
//!     Ok(n) | on!(reader.read(&mut buf)) if can_read => {
//!         if n == 0 { return Ok(()); }
//!         writer.write_all(&buf[..n]).await?;
//!     }
//!
//!     _ | on!(shutdown.recv()) => return Ok(()),
//! }
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
//! #[tokio_select(biased)]
//! match () {
//!     Some(Message::Data { id, payload }) | on!(rx.recv()) => {
//!         process(id, payload).await;
//!     }
//!
//!     _ => {
//!         println!("no messages pending");
//!         tokio::time::sleep(Duration::from_millis(50)).await;
//!     }
//! }
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
//!
//! # Requirements
//!
//! This crate requires nightly Rust, because custom attribute macros cannot currently be applied to expressions:
//!
//! ```
//! #![feature(proc_macro_hygiene)]
//! #![feature(stmt_expr_attributes)]
//! ```
//!
//! - [Tracking issue for `proc_macro_hygiene`](https://github.com/rust-lang/rust/issues/54727)
//! - [Tracking issue for `stmt_expr_attributes`](https://github.com/rust-lang/rust/issues/15701)
//!
//! # Design notes
//!
//! This section explains *why* that syntax is used.
//!
//! A single branch of the `tokio::select!` macro requires:
//!
//! - a pattern
//! - expression (the future)
//! - optional expression (the `if` condition)
//! - expression (handler)
//!
//! Using a custom DSL, such as `tokio::select!`, it's easy to come up with an arbitrary syntax that looks okay.
//!
//! But if we want `rustfmt` to work, then the expression must parse as valid Rust syntax. A `match` expression is *almost* perfect for this:
//!
//! ```
//! # /*
//! match {
//!     <pattern> (if <precondition>)? => <handler>,
//! }
//! # */
//! ```
//!
//! That covers:
//!
//! - ✅ a pattern
//! - ❌ expression (the future)
//! - ✅ optional expression (the `if` condition)
//! - ✅ expression (handler)
//!
//! We need to figure out how we can stuff an arbitrary expression into a match arm. Thankfully, macros
//! can expand to patterns, so we can abuse the fact that a match arm takes a `|`-separated list of "patterns":
//!
//! ```
//! # /*
//! match {
//!     <pattern> | on!(<future>) (if <precondition>)? => <handler>,
//! }
//! # */
//! ```
//!
//! And put whatever we need inside of the `on!` "macro", which is really a "fake macro" that does nothing,
//! the only purpose of the `on!` wrapper is that the `#[tokio_select]` attribute extracts all tokens
//! inside, and considers them an expression. Thus this:
//!
//! ```txt
//! <pattern> | on!(<future>) (if <precondition>)? => <handler>,
//! ```
//!
//! Is transformed into this:
//!
//! ```txt
//! <pattern> = <future> (e if <precondition>)? => <handler>,
//! ```
#![allow(rustdoc::invalid_rust_codeblocks)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;
use syn::Arm;
use syn::ExprMatch;
use syn::MacroDelimiter;
use syn::Pat;

/// Like `tokio::select!`, but formattable by `rustfmt`.
///
/// ```
/// # /*
/// #[tokio_select(biased)]
/// match () {
///     Some(Message::Data { id, payload }) | on!(rx.recv()) => {
///         process(id, payload).await;
///     }
///
///     _ => {
///         println!("no messages pending");
///         tokio::time::sleep(Duration::from_millis(50)).await;
///     }
/// }
/// # */
/// ```
///
/// See the [crate-level](crate) documentation for more info.
#[proc_macro_attribute]
pub fn tokio_select(args: TokenStream, input: TokenStream) -> TokenStream {
    let biased_kw = parse_macro_input!(args as Option<kw::biased>).into_iter();
    let mut select_arms = quote! { #(#biased_kw;)* };

    let input = parse_macro_input!(input as ExprMatch);

    for Arm {
        pat, guard, body, ..
    } in input.arms
    {
        match pat {
            Pat::Or(or) if or.cases.len() == 2 => {
                let pat = &or.cases[0];

                let precondition = guard.as_ref().map(|guard| &guard.1).into_iter();

                match &or.cases[1] {
                    Pat::Macro(macr)
                        if macr.mac.path.is_ident("on")
                            && matches!(macr.mac.delimiter, MacroDelimiter::Paren(_)) =>
                    {
                        let fut = &macr.mac.tokens;

                        select_arms.extend(quote! {
                            #pat = #fut #(, if #precondition)* => #body,
                        });
                    }
                    _ => {
                        return syn::Error::new_spanned(
                            pat,
                            "expected format: pattern | on!(future)",
                        )
                        .to_compile_error()
                        .into();
                    }
                }
            }
            Pat::Wild(_) => {
                select_arms.extend(quote! {
                    else => #body
                });
            }
            _ => {
                return syn::Error::new_spanned(pat, "expected format: pattern | on!(future)")
                    .to_compile_error()
                    .into();
            }
        }
    }

    quote! {
        ::tokio::select! {
            #select_arms
        }
    }
    .into()
}

mod kw {
    syn::custom_keyword!(biased);
}
