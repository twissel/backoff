#![feature(proc_macro)]
extern crate backoff;
extern crate futures;
use backoff::on_error;

#[on_error(max_tries = "5")]
fn dummy() -> impl futures::Future<Item = u32, Error = u32> {
    futures::future::ok(5)
}

#[test]
fn test_it_works() {
    let res = futures::executor::block_on(dummy());
    assert_eq!(res.is_ok(), true);
}
