#![doc = concat!("[![crates.io](https://img.shields.io/crates/v/", env!("CARGO_PKG_NAME"), "?style=flat-square&logo=rust)](https://crates.io/crates/", env!("CARGO_PKG_NAME"), ")")]
#![doc = concat!("[![docs.rs](https://img.shields.io/docsrs/", env!("CARGO_PKG_NAME"), "?style=flat-square&logo=docs.rs)](https://docs.rs/", env!("CARGO_PKG_NAME"), ")")]
#![doc = "![license](https://img.shields.io/badge/license-Apache--2.0_OR_MIT-blue?style=flat-square)"]
#![doc = concat!("![msrv](https://img.shields.io/badge/msrv-", env!("CARGO_PKG_RUST_VERSION"), "-blue?style=flat-square&logo=rust)")]
//! [![github](https://img.shields.io/github/stars/nik-rev/better-tokio-select)](https://github.com/nik-rev/better-tokio-select)
//!
//! This crate exports the macro [`tokio_select!`], which, unlike [`tokio::select!`](https://docs.rs/tokio/latest/tokio/macro.select.html), can be formatted by `rustfmt`!
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
//! `tokio_select!` takes a `match ..` expression as an argument, which has a list of arms:
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
//! For `rustfmt` to work, the argument to a macro must be a valid Rust expression. Hence the odd-looking `..`s.
//! Rust compiler expects a pattern in that position, and we provide it with one.
//!
//! Admittedly, the syntax is a little strange. But it's also formattable by `rustfmt`. Trade-offs, people, trade-offs!
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
//! `tokio_select!`:
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
//! `tokio_select!`:
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

// This documentation was adapted from the Tokio project for use with `tokio_select!` macro,
//
// https://github.com/tokio-rs/tokio/blob/ee4de818065a97c0be0d6bfc18fb36725349aef8/tokio/src/macros/select.rs
//
// License: MIT
//
// MIT License
//
// Copyright (c) Tokio Contributors
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//
/// Waits on multiple concurrent branches, returning when the **first** branch
/// completes, cancelling the remaining branches.
///
/// Has the same functionality as [`tokio::select!`], but can be formatted by `rustfmt`.
///
/// [`tokio::select!`]: https://docs.rs/tokio/latest/tokio/macro.select.html
///
/// The `tokio_select!` macro must be used inside of async functions, closures, and
/// blocks.
///
/// The `tokio_select!` macro accepts a `match` expression with the following pattern:
///
/// ```
/// # /*
/// tokio_select!(match .. {
///     .. if let <pattern> = <async expression> (&& <precondition>)? => <handler>,
///     // ..
/// })
/// # */
/// ```
///
/// Additionally, the `tokio_select!` macro may include a single, optional `_`
/// branch, which evaluates if none of the other branches match their patterns:
///
/// ```text
/// _ => <expression>
/// ```
///
/// The macro aggregates all `<async expression>` expressions and runs them
/// concurrently on the **current** task. Once the **first** expression
/// completes with a value that matches its `<pattern>`, the `tokio_select!` macro
/// returns the result of evaluating the completed branch's `<handler>`
/// expression.
///
/// Additionally, each branch may include an optional precondition after `&&`. If the
/// precondition returns `false`, then the branch is disabled. The provided
/// `<async expression>` is still evaluated but the resulting future is never
/// polled. This capability is useful when using `tokio_select!` within a loop.
///
/// The complete lifecycle of a `tokio_select!` expression is as follows:
///
/// 1. Evaluate all provided `<precondition>` expressions. If the precondition
///    returns `false`, disable the branch for the remainder of the current call
///    to `tokio_select!`. Re-entering `tokio_select!` due to a loop clears the "disabled"
///    state.
/// 2. Aggregate the `<async expression>`s from each branch, including the
///    disabled ones. If the branch is disabled, `<async expression>` is still
///    evaluated, but the resulting future is not polled.
/// 3. If **all** branches are disabled: go to step 6.
/// 4. Concurrently await on the results for all remaining `<async expression>`s.
/// 5. Once an `<async expression>` returns a value, attempt to apply the value to the
///    provided `<pattern>`. If the pattern matches, evaluate the `<handler>` and return.
///    If the pattern **does not** match, disable the current branch for the remainder of
///    the current call to `tokio_select!`. Continue from step 3.
/// 6. Evaluate the `_` (else) expression. If no else expression is provided, panic.
///
/// # Runtime characteristics
///
/// By running all async expressions on the current task, the expressions are
/// able to run **concurrently** but not in **parallel**. This means all
/// expressions are run on the same thread and if one branch blocks the thread,
/// all other expressions will be unable to continue. If parallelism is
/// required, spawn each async expression using [`tokio::spawn`] and pass the
/// join handle to `tokio_select!`.
///
/// [`tokio::spawn`]: https://docs.rs/tokio/latest/tokio/task/fn.spawn.html
///
/// # Fairness
///
/// By default, `tokio_select!` randomly picks a branch to check first. This provides
/// some level of fairness when calling `tokio_select!` in a loop with branches that
/// are always ready.
///
/// This behavior can be overridden by adding `biased,` to the beginning of the
/// macro input. See the examples for details. This will cause `tokio_select!` to poll
/// the futures in the order they appear from top to bottom. There are a few
/// reasons you may want this:
///
/// - The random number generation of `tokio_select!` has a non-zero CPU cost
/// - Your futures may interact in a way where known polling order is significant
///
/// But there is an important caveat to this mode. It becomes your responsibility
/// to ensure that the polling order of your futures is fair. If for example you
/// are selecting between a stream and a shutdown future, and the stream has a
/// huge volume of messages and zero or nearly zero time between them, you should
/// place the shutdown future earlier in the `tokio_select!` list to ensure that it is
/// always polled, and will not be ignored due to the stream being constantly
/// ready.
///
/// # Panics
///
/// The `tokio_select!` macro panics if all branches are disabled **and** there is no
/// provided `_` (else) branch. A branch is disabled when the provided `if`
/// precondition returns `false` **or** when the pattern does not match the
/// result of `<async expression>`.
///
/// # Cancellation safety
///
/// When using `tokio_select!` in a loop to receive messages from multiple sources,
/// you should make sure that the receive call is cancellation safe to avoid
/// losing messages. This section goes through various common methods and
/// describes whether they are cancel safe.  The lists in this section are not
/// exhaustive.
///
/// The following methods are cancellation safe:
///
///  * [`tokio::sync::mpsc::Receiver::recv`](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Receiver.html#method.recv)
///  * [`tokio::sync::mpsc::UnboundedReceiver::recv`](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.UnboundedReceiver.html#method.recv)
///  * [`tokio::sync::broadcast::Receiver::recv`](https://docs.rs/tokio/latest/tokio/sync/broadcast/struct.Receiver.html#method.recv)
///  * [`tokio::sync::watch::Receiver::changed`](https://docs.rs/tokio/latest/tokio/sync/watch/struct.Receiver.html#method.changed)
///  * [`tokio::net::TcpListener::accept`](https://docs.rs/tokio/latest/tokio/net/struct.TcpListener.html#method.accept)
///  * [`tokio::net::UnixListener::accept`](https://docs.rs/tokio/latest/tokio/net/struct.UnixListener.html#method.accept)
///  * [`tokio::signal::unix::Signal::recv`](https://docs.rs/tokio/latest/tokio/signal/unix/struct.Signal.html#method.recv)
///  * [`tokio::io::AsyncReadExt::read`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html#method.read) on any `AsyncRead`
///  * [`tokio::io::AsyncReadExt::read_buf`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html#method.read_buf) on any `AsyncRead`
///  * [`tokio::io::AsyncWriteExt::write`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWriteExt.html#method.write) on any `AsyncWrite`
///  * [`tokio::io::AsyncWriteExt::write_buf`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWriteExt.html#method.write_buf) on any `AsyncWrite`
///  * [`tokio_stream::StreamExt::next`]([https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.next](https://docs.rs/tokio-stream/0.1/tokio_stream/trait.StreamExt.html#method.next)) on any `Stream`
///  * [`futures::stream::StreamExt::next`]([https://docs.rs/futures/0.3/futures/stream/trait.StreamExt.html#method.next](https://docs.rs/futures/0.3/futures/stream/trait.StreamExt.html#method.next)) on any `Stream`
///
/// The following methods are not cancellation safe and can lead to loss of data:
///
///  * [`tokio::io::AsyncReadExt::read_exact`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html#method.read_exact)
///  * [`tokio::io::AsyncReadExt::read_to_end`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html#method.read_to_end)
///  * [`tokio::io::AsyncReadExt::read_to_string`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html#method.read_to_string)
///  * [`tokio::io::AsyncWriteExt::write_all`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWriteExt.html#method.write_all)
///
/// The following methods are not cancellation safe because they use a queue for
/// fairness and cancellation makes you lose your place in the queue:
///
///  * [`tokio::sync::Mutex::lock`](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html#method.lock)
///  * [`tokio::sync::RwLock::read`](https://docs.rs/tokio/latest/tokio/sync/struct.RwLock.html#method.read)
///  * [`tokio::sync::RwLock::write`](https://docs.rs/tokio/latest/tokio/sync/struct.RwLock.html#method.write)
///  * [`tokio::sync::Semaphore::acquire`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html#method.acquire)
///  * [`tokio::sync::Notify::notified`](https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html#method.notified)
///
/// To determine whether your own methods are cancellation safe, look for the
/// location of uses of `.await`. This is because when an asynchronous method is
/// cancelled, that always happens at an `.await`. If your function behaves
/// correctly even if it is restarted while waiting at an `.await`, then it is
/// cancellation safe.
///
/// Cancellation safety can be defined in the following way: If you have a
/// future that has not yet completed, then it must be a no-op to drop that
/// future and recreate it. This definition is motivated by the situation where
/// a `tokio_select!` is used in a loop. Without this guarantee, you would lose your
/// progress when another branch completes and you restart the `tokio_select!` by
/// going around the loop.
///
/// Be aware that cancelling something that is not cancellation safe is not
/// necessarily wrong. For example, if you are cancelling a task because the
/// application is shutting down, then you probably don't care that partially
/// read data is lost.
///
/// # Examples
///
/// Basic select with two branches.
///
/// ```
/// # /*
/// async fn do_stuff_async() {
///     // async work
/// }
///
/// async fn more_async_work() {
///     // more here
/// }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// tokio_select!(match .. {
///     .. if let _ = do_stuff_async() => {
///         println!("do_stuff_async() completed first")
///     }
///     .. if let _ = more_async_work() => {
///         println!("more_async_work() completed first")
///     }
/// });
/// # } */
/// ```
///
/// Basic stream selecting.
///
/// ```
/// # /*
/// use tokio_stream::{self as stream, StreamExt};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut stream1 = stream::iter(vec![1, 2, 3]);
/// let mut stream2 = stream::iter(vec![4, 5, 6]);
///
/// let next = tokio_select!(match .. {
///     .. if let v = stream1.next() => v.unwrap(),
///     .. if let v = stream2.next() => v.unwrap(),
/// });
///
/// assert!(next == 1 || next == 4);
/// # }
/// # */
/// ```
///
/// Collect the contents of two streams. In this example, we rely on pattern
/// matching and the fact that `stream::iter` is "fused", i.e. once the stream
/// is complete, all calls to `next()` return `None`.
///
/// ```
/// # /*
/// use tokio_stream::{self as stream, StreamExt};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut stream1 = stream::iter(vec![1, 2, 3]);
/// let mut stream2 = stream::iter(vec![4, 5, 6]);
///
/// let mut values = vec![];
///
/// loop {
///     tokio_select!(match .. {
///         .. if let Some(v) = stream1.next() => values.push(v),
///         .. if let Some(v) = stream2.next() => values.push(v),
///         _ => break,
///     });
/// }
///
/// values.sort();
/// assert_eq!(&[1, 2, 3, 4, 5, 6], &values[..]);
/// # } */
/// ```
///
/// Using the same future in multiple `tokio_select!` expressions can be done by passing
/// a reference to the future. Doing so requires the future to be [`Unpin`]. A
/// future can be made [`Unpin`] by either using [`Box::pin`] or stack pinning.
///
/// [`Unpin`]: std::marker::Unpin
/// [`Box::pin`]: std::boxed::Box::pin
///
/// Here, a stream is consumed for at most 1 second.
///
/// ```
/// # /*
/// use tokio_stream::{self as stream, StreamExt};
/// use tokio::time::{self, Duration};
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut stream = stream::iter(vec![1, 2, 3]);
/// let sleep = time::sleep(Duration::from_secs(1));
/// tokio::pin!(sleep);
///
/// loop {
///     tokio_select!(match .. {
///         .. if let maybe_v = stream.next() => {
///             if let Some(v) = maybe_v {
///                 println!("got = {}", v);
///             } else {
///                 break;
///             }
///         }
///         .. if let _ = &mut sleep => {
///             println!("timeout");
///             break;
///         }
///     });
/// }
/// # } */
/// ```
///
/// Joining two values using `tokio_select!`.
///
/// ```
/// # /*
/// use tokio::sync::oneshot;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let (tx1, mut rx1) = oneshot::channel();
/// let (tx2, mut rx2) = oneshot::channel();
///
/// tokio::spawn(async move {
///     tx1.send("first").unwrap();
/// });
///
/// tokio::spawn(async move {
///     tx2.send("second").unwrap();
/// });
///
/// let mut a = None;
/// let mut b = None;
///
/// while a.is_none() || b.is_none() {
///     tokio_select!(match .. {
///         .. if let v1 = (&mut rx1), if a.is_none() => a = Some(v1.unwrap()),
///         .. if let v2 = (&mut rx2), if b.is_none() => b = Some(v2.unwrap()),
///     });
/// }
///
/// let res = (a.unwrap(), b.unwrap());
///
/// assert_eq!(res.0, "first");
/// assert_eq!(res.1, "second");
/// # } */
/// ```
///
/// Using the `biased,` mode to control polling order.
///
/// ```
/// # /* #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let mut count = 0u8;
///
/// loop {
///     tokio_select!(biased, match .. {
///         // If you run this example without `biased,`, the polling order is
///         // pseudo-random, and the assertions on the value of count will
///         // (probably) fail.
///         .. if let _ = async {}, if count < 1 => {
///             count += 1;
///             assert_eq!(count, 1);
///         }
///         .. if let _ = async {}, if count < 2 => {
///             count += 1;
///             assert_eq!(count, 2);
///         }
///         .. if let _ = async {}, if count < 3 => {
///             count += 1;
///             assert_eq!(count, 3);
///         }
///         .. if let _ = async {}, if count < 4 => {
///             count += 1;
///             assert_eq!(count, 4);
///         }
///         _ => {
///             break;
///         }
///     });
/// }
/// # } */
/// ```
///
/// ## Avoid racy `&&` preconditions
///
/// Given that `&&` preconditions are used to disable `tokio_select!` branches, some
/// caution must be used to avoid missing values.
///
/// For example, here is **incorrect** usage of `sleep` with `&&`. The objective
/// is to repeatedly run an asynchronous task for up to 50 milliseconds.
/// However, there is a potential for the `sleep` completion to be missed.
///
/// ```
/// # /*
/// use tokio::time::{self, Duration};
///
/// async fn some_async_work() {
///     // do work
/// }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let sleep = time::sleep(Duration::from_millis(50));
/// tokio::pin!(sleep);
///
/// while !sleep.is_elapsed() {
///     tokio_select!(match .. {
///         .. if let _ = &mut sleep && !sleep.is_elapsed() => {
///             println!("operation timed out");
///         }
///         .. if let _ = some_async_work() => {
///             println!("operation completed");
///         }
///     });
/// }
///
/// panic!("This example shows how not to do it!");
/// # } */
/// ```
///
/// In the above example, `sleep.is_elapsed()` may return `true` even if
/// `sleep.poll()` never returned `Ready`. This opens up a potential race
/// condition where `sleep` expires between the `while !sleep.is_elapsed()`
/// check and the call to `tokio_select!` resulting in the `some_async_work()` call to
/// run uninterrupted despite the sleep having elapsed.
///
/// One way to write the above example without the race would be:
///
/// ```
/// # /*
/// use tokio::time::{self, Duration};
///
/// async fn some_async_work() {
/// # time::sleep(Duration::from_millis(10)).await;
///     // do work
/// }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let sleep = time::sleep(Duration::from_millis(50));
/// tokio::pin!(sleep);
///
/// loop {
///     tokio_select!(match .. {
///         .. if let _ = &mut sleep => {
///             println!("operation timed out");
///             break;
///         }
///         .. if let _ = some_async_work() => {
///             println!("operation completed");
///         }
///     });
/// }
/// # } */
/// ```
///
/// # Alternatives from the Ecosystem
///
/// The `tokio_select!` macro is a powerful tool for managing multiple asynchronous
/// branches, enabling tasks to run concurrently within the same thread. However,
/// its use can introduce challenges, particularly around cancellation safety, which
/// can lead to subtle and hard-to-debug errors. For many use cases, ecosystem
/// alternatives may be preferable as they mitigate these concerns by offering
/// clearer syntax, more predictable control flow, and reducing the need to manually
/// handle issues like fuse semantics or cancellation safety.
///
/// ## Merging Streams
///
/// For cases where `loop { tokio_select! { ... } }` is used to poll multiple tasks,
/// stream merging offers a concise alternative, inherently handle cancellation-safe
/// processing, removing the risk of data loss. Libraries such as [`tokio_stream`],
/// [`futures::stream`] and [`futures_concurrency`] provide tools for merging
/// streams and handling their outputs sequentially.
///
/// [`tokio_stream`]: [https://docs.rs/tokio-stream/latest/tokio_stream/](https://docs.rs/tokio-stream/latest/tokio_stream/)
/// [`futures::stream`]: [https://docs.rs/futures/latest/futures/stream/](https://docs.rs/futures/latest/futures/stream/)
/// [`futures_concurrency`]: [https://docs.rs/futures-concurrency/latest/futures_concurrency/](https://docs.rs/futures-concurrency/latest/futures_concurrency/)
///
/// ### Example with `tokio_select!`
///
/// ```
/// # /*
/// struct File;
/// struct Channel;
/// struct Socket;
///
/// impl Socket {
///     async fn read_packet(&mut self) -> Vec<u8> {
///         vec![]
///     }
/// }
///
/// async fn read_send(_file: &mut File, _channel: &mut Channel) {
///     // do work that is not cancel safe
/// }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// // open our IO types
/// let mut file = File;
/// let mut channel = Channel;
/// let mut socket = Socket;
///
/// loop {
///     tokio_select!(match .. {
///         .. if let _ = read_send(&mut file, &mut channel) => { /* ... */ },
///         .. if let _data = socket.read_packet() => { /* ... */ }
///         _ => break
///     });
/// }
/// # } */
/// ```
///
/// ### Moving to `merge`
///
/// By using merge, you can unify multiple asynchronous tasks into a single stream,
/// eliminating the need to manage tasks manually and reducing the risk of
/// unintended behavior like data loss.
///
/// ```
/// # /*
/// use std::pin::pin;
///
/// use futures::stream::unfold;
/// use tokio_stream::StreamExt;
///
/// struct File;
/// struct Channel;
/// struct Socket;
///
/// impl Socket {
///     async fn read_packet(&mut self) -> Vec<u8> {
///         vec![]
///     }
/// }
///
/// async fn read_send(_file: &mut File, _channel: &mut Channel) {
///     // do work that is not cancel safe
/// }
///
/// enum Message {
///     Stop,
///     Sent,
///     Data(Vec<u8>),
/// }
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// // open our IO types
/// let file = File;
/// let channel = Channel;
/// let socket = Socket;
///
/// let a = unfold((file, channel), |(mut file, mut channel)| async {
///     read_send(&mut file, &mut channel).await;
///     Some((Message::Sent, (file, channel)))
/// });
/// let b = unfold(socket, |mut socket| async {
///     let data = socket.read_packet().await;
///     Some((Message::Data(data), socket))
/// });
/// let c = tokio_stream::iter([Message::Stop]);
///
/// let mut s = pin!(a.merge(b).merge(c));
/// while let Some(msg) = s.next().await {
///     match msg {
///         Message::Data(_data) => { /* ... */ }
///         Message::Sent => continue,
///         Message::Stop => break,
///     }
/// }
/// # }
/// # */
/// ```
///
/// ## Racing Futures
///
/// If you need to wait for the first completion among several asynchronous tasks,
/// ecosystem utilities such as
/// [`futures`]([https://docs.rs/futures/latest/futures/](https://docs.rs/futures/latest/futures/)),
/// [`futures-lite`]([https://docs.rs/futures-lite/latest/futures_lite/](https://docs.rs/futures-lite/latest/futures_lite/)) or
/// [`futures-concurrency`]([https://docs.rs/futures-concurrency/latest/futures_concurrency/](https://docs.rs/futures-concurrency/latest/futures_concurrency/))
/// provide streamlined syntax for racing futures:
///
/// - [`futures_concurrency::future::Race`]([https://docs.rs/futures-concurrency/latest/futures_concurrency/future/trait.Race.html](https://docs.rs/futures-concurrency/latest/futures_concurrency/future/trait.Race.html))
/// - [`futures::select`]([https://docs.rs/futures/latest/futures/macro.select.html](https://docs.rs/futures/latest/futures/macro.select.html))
/// - [`futures::stream::select_all`]([https://docs.rs/futures/latest/futures/stream/select_all/index.html](https://docs.rs/futures/latest/futures/stream/select_all/index.html)) (for streams)
/// - [`futures_lite::future::or`]([https://docs.rs/futures-lite/latest/futures_lite/future/fn.or.html](https://docs.rs/futures-lite/latest/futures_lite/future/fn.or.html))
/// - [`futures_lite::future::race`]([https://docs.rs/futures-lite/latest/futures_lite/future/fn.race.html](https://docs.rs/futures-lite/latest/futures_lite/future/fn.race.html))
///
/// ```
/// # /*
/// use futures_concurrency::future::Race;
///
/// # #[tokio::main(flavor = "current_thread")]
/// # async fn main() {
/// let task_a = async { Ok("ok") };
/// let task_b = async { Err("error") };
/// let result = (task_a, task_b).race().await;
///
/// match result {
///     Ok(output) => println!("First task completed with: {output}"),
///     Err(err) => eprintln!("Error occurred: {err}"),
/// }
/// # }
/// # */
/// ```
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
