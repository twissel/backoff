#![feature(use_extern_macros)]
extern crate backoff_macro;
extern crate futures;

pub use backoff_macro::on_error;
pub use futures::{Async, Future};

use std::time::{Duration, Instant};

enum BackoffState<Fut>
where
    Fut: Future,
{
    Initial,
    Processing(StateProcessing<Fut>),
    Finished(Result<Fut::Item, Fut::Error>),
    Temp,
}

struct StateProcessing<Fut> {
    fut: Fut,
    started: Instant,
    tried: u64,
}

pub struct Backoff<Fut>
where
    Fut: Future,
{
    state: BackoffState<Fut>,
    func: Box<Fn() -> Fut>,
    max_tries: Option<u64>,
    max_time: Option<Duration>,
}

impl<Fut> Backoff<Fut>
where
    Fut: Future,
{
    pub fn new<F: Fn() -> Fut + 'static>(
        max_tries: Option<u64>,
        max_time: Option<u64>,
        func: F,
    ) -> Self {
        let max_time = max_time.map(|secs| Duration::from_secs(secs as u64));
        let state = BackoffState::Initial;
        let func = Box::new(func);
        Backoff {
            state,
            func,
            max_tries,
            max_time,
        }
    }
}

impl<Fut> Future for Backoff<Fut>
where
    Fut: Future,
{
    type Item = Fut::Item;
    type Error = Fut::Error;

    fn poll(
        &mut self,
        ctx: &mut futures::task::Context,
    ) -> Result<futures::Async<Self::Item>, Self::Error> {
        loop {
            let current_state = std::mem::replace(&mut self.state, BackoffState::Temp);
            match current_state {
                BackoffState::Initial => {
                    let fut = (self.func)();
                    let started = Instant::now();
                    let tried = 0;
                    self.state = BackoffState::Processing(StateProcessing {
                        fut,
                        started,
                        tried,
                    });
                }
                BackoffState::Processing(mut processing) => {
                    let poll = processing.fut.poll(ctx);
                    match poll {
                        Ok(Async::Ready(result)) => {
                            self.state = BackoffState::Finished(Ok(result));
                        }
                        Ok(Async::Pending) => {
                            self.state = BackoffState::Processing(processing);
                            return Ok(Async::Pending);
                        }
                        Err(e) => {
                            println!("future failed");
                            processing.tried += 1;
                            if let Some(max_time) = self.max_time {
                                let elapsed = processing.started.elapsed();
                                if elapsed > max_time {
                                    self.state = BackoffState::Finished(Err(e));
                                    continue;
                                }
                            }

                            if let Some(max_tries) = self.max_tries {
                                if processing.tried >= max_tries {
                                    self.state = BackoffState::Finished(Err(e));
                                    continue;
                                }
                            }
                            processing.fut = (self.func)();
                            self.state = BackoffState::Processing(processing);
                        }
                    }
                }
                BackoffState::Finished(result) => match result {
                    Ok(val) => return Ok(Async::Ready(val)),
                    Err(e) => return Err(e),
                },
                BackoffState::Temp => unreachable!("BackoffState::Temp reached!"),
            }
        }
    }
}
