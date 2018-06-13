#![feature(proc_macro)]
extern crate backoff;
extern crate futures;
use backoff::on_error;

#[on_error(max_tries = "5")]
unsafe fn dummy(param: &'static mut u64) -> futures::future::FutureResult<u32, u32> {
    println!("dummy called");
    if *param > 0 {
        *param -= 1;
        futures::future::err(5)
    } else {
        futures::future::ok(5)
    }
}

static mut VAL: u64 = 4;

#[test]
fn test_it_works() {
    unsafe {
        let res = futures::executor::block_on(dummy(&mut VAL));
        assert_eq!(res.is_ok(), true);
    }
}
