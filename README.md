# `better_tokio_select`

<!-- cargo-reedme: start -->

<!-- cargo-reedme: info-start

    Do not edit this region by hand
    ===============================

    This region was generated from Rust documentation comments by `cargo-reedme` using this command:

        cargo +nightly reedme

    for more info: https://github.com/nik-rev/cargo-reedme

cargo-reedme: info-end -->

[![crates.io](https://img.shields.io/crates/v/better_tokio_select?style=flat-square&logo=rust)](https://crates.io/crates/better_tokio_select)
[![docs.rs](https://img.shields.io/docsrs/better_tokio_select?style=flat-square&logo=docs.rs)](https://docs.rs/better_tokio_select)
![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)
![msrv](https://img.shields.io/badge/msrv-nightly-blue?style=flat-square&logo=rust)
[![github](https://img.shields.io/github/stars/nik-rev/better-tokio-select)](https://github.com/nik-rev/better-tokio-select)

This crate exports the macro [`#[tokio_select]`](https://docs.rs/better_tokio_select/latest/better_tokio_select/attr.tokio_select.html), which, unlike [`tokio::select!`](https://docs.rs/tokio/latest/tokio/macro.select.html), can be formatted by `rustfmt`!

```toml
better_tokio_select = "0.1"
```

## Syntax

This macro has all the same capabilities as `tokio::select!`, but the syntax is *slightly* different.

`tokio::select!` takes a list of branches:

```txt
<pattern> = <async expression> (, if <precondition>)? => <handler>,
```

Example:

```rust
tokio::select! {
    Ok(res) = reader.read(&mut buf), if can_read => {
        writer.write_all(res.bytes)
    }
}
```

`#[tokio_select]` applies to a `match` expression, which has a list of arms:

```txt
() if let <pattern> = <async expression> (&& <precondition>)? => <handler>,
```

Example:

```rust
#[tokio_select]
match () {
    () if let Ok(res) = reader.read(&mut buf) && can_read => {
        writer.write_all(res.bytes)
    }
}
```

## Examples

### TCP Proxy with Cancellation and Guard

`tokio::select!`:

```rust
tokio::select! {
    res = reader.read(&mut buf), if can_read => {
        let n = res?;
        if n == 0 { return Ok(()); }
        writer.write_all(&buf[..n]).await?;
    }

    _ = shutdown.recv() => {
        return Ok(());
    }
}
```

`#[tokio_select]`:

```rust
#[tokio_select]
match () {
    () if let Ok(n) = reader.read(&mut buf) && can_read => {
        let n = res?;
        if n == 0 { return Ok(()); }
        writer.write_all(&buf[..n]).await?;
    }

    () if let _ = shutdown.recv() => {
        return Ok(())
    }
}
```

Admittedly, the syntax is a little strange. But it’s also formattable by `rustfmt`. Trade-offs, people, trade-offs!

### Rate-Limited Message Processor

```rust
tokio::select! {
    biased;

    Some(Message::Data { id, payload }) = rx.recv() => {
        process(id, payload).await;
    }

    else => {
        println!("no messages pending");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
```

`#[tokio_select]`:

```rust
#[tokio_select(biased)]
match () {
    () if let Some(Message::Data { id, payload }) = rx.recv() => {
        process(id, payload).await;
    }

    _ => {
        println!("no messages pending");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
```

## Global import

You can make the `tokio_select!` macro globally available in your crate, without needing to import it, with:

```rust
#[macro_use(tokio_select)]
extern crate better_tokio_select;
```

## Requirements

This crate requires nightly Rust, because custom attribute macros cannot currently be applied to expressions:

```rust
#![feature(proc_macro_hygiene)]
#![feature(stmt_expr_attributes)]
```

- [Tracking issue for `proc_macro_hygiene`](https://github.com/rust-lang/rust/issues/54727)
- [Tracking issue for `stmt_expr_attributes`](https://github.com/rust-lang/rust/issues/15701)

<!-- cargo-reedme: end -->
