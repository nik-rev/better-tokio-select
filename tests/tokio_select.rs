//! These tests are adapted from the `tokio` project's `macro_select.rs` file, which tests
//! `tokio::select!` macro. Every invocation of `tokio::select!` was replaced with `#[tokio_select]`.
//!
//! https://github.com/tokio-rs/tokio/blob/ee4de818065a97c0be0d6bfc18fb36725349aef8/tokio/tests/macros_select.rs
//!
//! License: MIT
//!
//! MIT License
//!
//! Copyright (c) Tokio Contributors
//!
//! Permission is hereby granted, free of charge, to any person obtaining a copy
//! of this software and associated documentation files (the "Software"), to deal
//! in the Software without restriction, including without limitation the rights
//! to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
//! copies of the Software, and to permit persons to whom the Software is
//! furnished to do so, subject to the following conditions:
//!
//! The above copyright notice and this permission notice shall be included in all
//! copies or substantial portions of the Software.
//!
//! THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//! IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//! FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//! AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//! LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
//! OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
//! SOFTWARE.
#![allow(clippy::disallowed_names)]

use std::future::poll_fn;
use std::task::Poll::Ready;

use better_tokio_select::tokio_select;
use tokio::sync::oneshot;
use tokio::test as maybe_tokio_test;
use tokio_test::assert_ok;
use tokio_test::assert_pending;
use tokio_test::assert_ready;

#[maybe_tokio_test]
async fn sync_one_lit_expr_comma() {
    let foo = tokio_select!(match .. {
        .. if let foo = async { 1 } => foo,
    });

    assert_eq!(foo, 1);
}

#[maybe_tokio_test]
async fn no_branch_else_only() {
    let foo = tokio_select!(match .. {
        _ => 1,
    });

    assert_eq!(foo, 1);
}

#[maybe_tokio_test]
async fn no_branch_else_only_biased() {
    let foo = tokio_select!(biased, match .. {
        _ => 1,
    });

    assert_eq!(foo, 1);
}

#[maybe_tokio_test]
async fn nested_one() {
    let foo = tokio_select!(match .. {
        .. if let foo = async { 1 } => {
            tokio_select!(match .. {
                .. if let bar = async { foo } => bar,
            })
        }
    });

    assert_eq!(foo, 1);
}

#[maybe_tokio_test]
async fn sync_one_lit_expr_no_comma() {
    let foo = tokio_select!(match .. {
        .. if let foo = async { 1 } => foo,
    });

    assert_eq!(foo, 1);
}

#[maybe_tokio_test]
async fn sync_one_lit_expr_block() {
    let foo = tokio_select!(match .. {
        .. if let foo = async { 1 } => foo,
    });

    assert_eq!(foo, 1);
}

#[maybe_tokio_test]
async fn sync_one_await() {
    let foo = tokio_select!(match .. {
        .. if let foo = one() => foo,
    });

    assert_eq!(foo, 1);
}

#[maybe_tokio_test]
async fn sync_one_ident() {
    let one = one();

    let foo = tokio_select!(match .. {
        .. if let foo = one => foo,
    });

    assert_eq!(foo, 1);
}

#[maybe_tokio_test]
async fn sync_two() {
    use std::cell::Cell;

    let cnt = Cell::new(0);

    let res = tokio_select!(match .. {
        .. if let foo = async {
            cnt.set(cnt.get() + 1);
            1
        } =>
        {
            foo
        }
        .. if let bar = async {
            cnt.set(cnt.get() + 1);
            2
        } =>
        {
            bar
        }
    });

    assert_eq!(1, cnt.get());
    assert!(res == 1 || res == 2);
}

#[maybe_tokio_test]
async fn drop_in_fut() {
    let s = "hello".to_string();

    let res = tokio_select!(match .. {
        .. if let foo = async {
            let v = one().await;
            drop(s);
            v
        } =>
        {
            foo
        }
    });

    assert_eq!(res, 1);
}

#[maybe_tokio_test]
async fn one_ready() {
    let (tx1, rx1) = oneshot::channel::<i32>();
    let (_tx2, rx2) = oneshot::channel::<i32>();

    tx1.send(1).unwrap();

    let v = tokio_select!(match .. {
        .. if let res = rx1 => assert_ok!(res),
        .. if let _ = rx2 => unreachable!(),
    });

    assert_eq!(1, v);
}

#[maybe_tokio_test]
async fn select_streams() {
    use tokio::sync::mpsc;

    let (tx1, mut rx1) = mpsc::unbounded_channel::<i32>();
    let (tx2, mut rx2) = mpsc::unbounded_channel::<i32>();

    tokio::spawn(async move {
        assert_ok!(tx2.send(1));
        tokio::task::yield_now().await;
        assert_ok!(tx1.send(2));
        tokio::task::yield_now().await;
        assert_ok!(tx2.send(3));
        tokio::task::yield_now().await;
    });

    let mut rem = true;
    let mut msgs = vec![];

    while rem {
        tokio_select!(match .. {
            .. if let Some(x) = rx1.recv() => msgs.push(x),
            .. if let Some(y) = rx2.recv() => msgs.push(y),
            _ => rem = false,
        })
    }

    msgs.sort_unstable();
    assert_eq!(&msgs[..], &[1, 2, 3]);
}

#[maybe_tokio_test]
async fn move_uncompleted_futures() {
    let (tx1, mut rx1) = oneshot::channel::<i32>();
    let (tx2, mut rx2) = oneshot::channel::<i32>();

    tx1.send(1).unwrap();
    tx2.send(2).unwrap();

    let ran;

    tokio_select!(match .. {
        .. if let res = &mut rx1 => {
            assert_eq!(1, assert_ok!(res));
            assert_eq!(2, assert_ok!(rx2.await));
            ran = true;
        }
        .. if let res = &mut rx2 => {
            assert_eq!(2, assert_ok!(res));
            assert_eq!(1, assert_ok!(rx1.await));
            ran = true;
        }
    });

    assert!(ran);
}

#[maybe_tokio_test]
async fn nested() {
    let res = tokio_select!(match .. {
        .. if let x = async { 1 } => {
            tokio_select!(match .. {
                .. if let y = async { 2 } => x + y,
            })
        }
    });

    assert_eq!(res, 3);
}

#[cfg(target_pointer_width = "64")]
mod pointer_64_tests {
    use std::mem;

    use futures::future;

    use super::maybe_tokio_test;
    use super::*;

    #[maybe_tokio_test]
    async fn struct_size_1() {
        let fut = async {
            let ready = future::ready(0i32);
            tokio_select!(match .. {
                .. if let _ = ready => {}
            })
        };
        assert_eq!(mem::size_of_val(&fut), 32);
    }

    #[maybe_tokio_test]
    async fn struct_size_2() {
        let fut = async {
            let ready1 = future::ready(0i32);
            let ready2 = future::ready(0i32);
            tokio_select!(match .. {
                .. if let _ = ready1 => {}
                .. if let _ = ready2 => {}
            })
        };
        assert_eq!(mem::size_of_val(&fut), 40);
    }

    #[maybe_tokio_test]
    async fn struct_size_3() {
        let fut = async {
            let ready1 = future::ready(0i32);
            let ready2 = future::ready(0i32);
            let ready3 = future::ready(0i32);
            tokio_select!(match .. {
                .. if let _ = ready1 => {}
                .. if let _ = ready2 => {}
                .. if let _ = ready3 => {}
            })
        };
        assert_eq!(mem::size_of_val(&fut), 48);
    }
}

#[maybe_tokio_test]
async fn mutable_borrowing_future_with_same_borrow_in_block() {
    let mut value = 234;

    tokio_select!(match .. {
        .. if let _ = require_mutable(&mut value) => {}
        .. if let _ = async_noop() => {
            value += 5;
        }
    });

    assert!(value >= 234);
}

#[maybe_tokio_test]
async fn mutable_borrowing_future_with_same_borrow_in_block_and_else() {
    let mut value = 234;

    tokio_select!(match .. {
        .. if let _ = require_mutable(&mut value) => {}
        .. if let _ = async_noop() => {
            value += 5;
        }
        _ => {
            value += 27;
        }
    });

    assert!(value >= 234);
}

#[maybe_tokio_test]
async fn future_panics_after_poll() {
    use tokio_test::task;
    let (tx, rx) = oneshot::channel();
    let mut polled = false;

    let f = poll_fn(|_| {
        assert!(!polled);
        polled = true;
        Ready(None::<()>)
    });

    let mut f = task::spawn(async {
        tokio_select!(match .. {
            .. if let Some(_) = f => unreachable!(),
            .. if let ret = rx => ret.unwrap(),
        })
    });

    assert_pending!(f.poll());
    assert_pending!(f.poll());
    assert_ok!(tx.send(1));
    let res = assert_ready!(f.poll());
    assert_eq!(1, res);
}

#[maybe_tokio_test]
async fn disable_with_if() {
    use tokio_test::task;
    let f = poll_fn(|_| panic!());
    let (tx, rx) = oneshot::channel();

    let mut f = task::spawn(async {
        tokio_select!(match .. {
            .. if let _ = f
                && false =>
            {
                unreachable!()
            }
            .. if let _ = rx => (),
        })
    });

    assert_pending!(f.poll());
    assert_ok!(tx.send(()));
    assert!(f.is_woken());
    assert_ready!(f.poll());
}

#[maybe_tokio_test]
async fn join_with_select() {
    use tokio_test::task;
    let (tx1, mut rx1) = oneshot::channel();
    let (tx2, mut rx2) = oneshot::channel();

    let mut f = task::spawn(async {
        let mut a = None;
        let mut b = None;
        while a.is_none() || b.is_none() {
            tokio_select!(match .. {
                .. if let v1 = &mut rx1
                    && a.is_none() =>
                {
                    a = Some(assert_ok!(v1))
                }
                .. if let v2 = &mut rx2
                    && b.is_none() =>
                {
                    b = Some(assert_ok!(v2))
                }
            })
        }
        (a.unwrap(), b.unwrap())
    });

    assert_pending!(f.poll());
    assert_ok!(tx1.send(123));
    assert_pending!(f.poll());
    assert_ok!(tx2.send(456));
    let (a, b) = assert_ready!(f.poll());
    assert_eq!(a, 123);
    assert_eq!(b, 456);
}

#[tokio::test]
async fn use_future_in_if_condition() {
    use tokio::time::Duration;
    use tokio::time::{self};
    tokio_select!(match .. {
        .. if let _ = time::sleep(Duration::from_millis(10))
            && false =>
        {
            panic!("if condition ignored")
        }
        .. if let _ = async { 1u32 } => {}
    })
}

#[tokio::test]
async fn use_future_in_if_condition_biased() {
    use tokio::time::Duration;
    use tokio::time::{self};
    tokio_select!(biased, match .. {
        .. if let _ = time::sleep(Duration::from_millis(10))
            && false =>
        {
            panic!("if condition ignored")
        }
        .. if let _ = async { 1u32 } => {}
    })
}

#[maybe_tokio_test]
async fn many_branches() {
    let num = tokio_select!(match .. {
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
        .. if let x = async { 1 } => x,
    });
    assert_eq!(1, num);
}

#[maybe_tokio_test]
async fn never_branch_no_warnings() {
    let t = tokio_select!(match .. {
        .. if let _ = async_never() => 0,
        .. if let one_async_ready = one() => one_async_ready,
    });
    assert_eq!(t, 1);
}

#[maybe_tokio_test]
async fn mut_on_left_hand_side() {
    let v = async move {
        let ok = async { 1 };
        tokio::pin!(ok);
        tokio_select!(match .. {
            .. if let mut a = &mut ok => {
                a += 1;
                a
            }
        })
    }
    .await;
    assert_eq!(v, 2);
}

#[maybe_tokio_test]
async fn biased_one_not_ready() {
    let (_tx1, rx1) = oneshot::channel::<i32>();
    let (tx2, rx2) = oneshot::channel::<i32>();
    let (tx3, rx3) = oneshot::channel::<i32>();

    tx2.send(2).unwrap();
    tx3.send(3).unwrap();

    let v = tokio_select!(biased, match .. {
        .. if let _ = rx1 => unreachable!(),
        .. if let res = rx2 => assert_ok!(res),
        .. if let _ = rx3 => panic!("Branch failure"),
    });
    assert_eq!(2, v);
}

#[maybe_tokio_test]
async fn biased_eventually_ready() {
    use tokio::task::yield_now;
    let one = async {};
    let two = async { yield_now().await };
    let three = async { yield_now().await };
    let mut count = 0u8;
    tokio::pin!(one, two, three);

    loop {
        tokio_select!(biased, match .. {
            .. if let _ = &mut two
                && count < 2 =>
            {
                count += 1;
                assert_eq!(count, 2);
            }
            .. if let _ = &mut three
                && count < 3 =>
            {
                count += 1;
                assert_eq!(count, 3);
            }
            .. if let _ = &mut one
                && count < 1 =>
            {
                count += 1;
                assert_eq!(count, 1);
            }
            _ => break,
        })
    }
    assert_eq!(count, 3);
}

#[maybe_tokio_test]
async fn mut_ref_patterns() {
    tokio_select!(match .. {
        .. if let Some(mut foo) = async { Some("1".to_string()) } => {
            assert_eq!(foo, "1");
            foo = "2".to_string();
            assert_eq!(foo, "2");
        }
    });

    tokio_select!(match .. {
        .. if let Some(ref foo) = async { Some("1".to_string()) } => {
            assert_eq!(*foo, "1");
        }
    });

    tokio_select!(match .. {
        .. if let Some(ref mut foo) = async { Some("1".to_string()) } => {
            assert_eq!(*foo, "1");
            *foo = "2".to_string();
            assert_eq!(*foo, "2");
        }
    });
}

#[tokio::test]
async fn select_into_future() {
    struct NotAFuture;
    impl std::future::IntoFuture for NotAFuture {
        type Output = ();
        type IntoFuture = std::future::Ready<()>;
        fn into_future(self) -> Self::IntoFuture {
            std::future::ready(())
        }
    }

    tokio_select!(match .. {
        .. if let () = NotAFuture => {}
    })
}

#[tokio::test]
async fn temporary_lifetime_extension() {
    tokio_select!(match .. {
        .. if let () = &mut std::future::ready(()) => {}
    })
}

#[tokio::test]
async fn select_is_budget_aware() {
    const BUDGET: usize = 128;
    let task = || {
        Box::pin(async move {
            tokio_select!(biased, match .. {
                .. if let () = tokio::task::coop::consume_budget() => {}
                .. if let () = std::future::ready(()) => {}
            })
        })
    };

    for _ in 0..BUDGET {
        assert!(futures::poll!(&mut task()).is_ready());
    }
    assert!(futures::poll!(&mut task()).is_pending());
}

async fn one() -> usize {
    1
}
async fn require_mutable(_: &mut i32) {}
async fn async_noop() {}
async fn async_never() -> ! {
    futures::future::pending().await
}
