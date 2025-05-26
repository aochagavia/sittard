Sittard
=======

[![Documentation](https://docs.rs/sittard/badge.svg)](https://docs.rs/sittard/)
[![Crates.io](https://img.shields.io/crates/v/sittard.svg)](https://crates.io/crates/sittard)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE-APACHE)

A **S**ans-**I**O **t**ickless **a**sync **r**untime, fully **d**eterministic.

That's a mouthful, so let's unpack it:

- Async runtime: sittard runs async Rust code, i.e. stuff that implements the `Future` trait.
- Sans-IO: sittard doesn't support asynchronous IO (e.g. network requests, filesystem operations,
  etc).
- Tickless: sittard allows async code to "sleep", but instead of waiting for the time to elapse,
  sittard advances its virtual clock whenever necessary.
- Fully deterministic: running the same code under sittard always yields the same results, unless
  the async code itself is a source of non-determinism.

## Example

The following code completes instantly, even though it "sleeps" for 60 seconds:

```rust
fn main() {
    // Create a runtime and run a future
    let rt = Runtime::default();
    rt.block_on(async move {
        let now = sittard::time::Instant::now();
        sittard::time::sleep(Duration::from_secs(60)).await;
        let elapsed_secs = now.elapsed().as_secs_f64();
        println!("Here we are, {elapsed_secs} seconds later...");
    });
}
```

See [`sittard/examples`](./sittard/examples/) for more.

## But why?

Sittard was born out of the need to [simulate QUIC network traffic in deep
space](https://github.com/aochagavia/quinn-workbench), where the delay between nodes goes from a few
minutes (e.g. Earth-Mars) to longer than a day (e.g. Earth-Voyager). Running these simulations in a
traditional way would require long waits, potentially multiple days! Fortunately, with sittard we
can run thousands of simulated hours in mere seconds. An additional bonus is that deterministic
execution ensures reproducible results across runs.

Note that sittard is unsuitable for common async scenarios, such as programming a web server or
accessing an API over a network.

## Differences with tokio

The tokio async runtime also supports a tickless and deterministic mode of operation, which is meant
for testing. As of this writing, you can enable it through
[`runtime::Builder::start_paused`](https://docs.rs/tokio/1.45.1/tokio/runtime/struct.Builder.html#method.start_paused)
or through [`time::pause`](https://docs.rs/tokio/1.45.1/tokio/time/fn.pause.html).

Sittard offers similar functionality, but allows the user to control the method to advance the
internal clock through the `AdvanceClock` trait. Users can implement the trait in whichever way they see fit, but the crate already provides two implementations:

1. `AdvanceToNextWake`: when the runtime cannot make any progress (i.e. all futures are blocked), it
   advances to the exact moment upon which the next sleep elapses.
2. `AdvanceToNextWakeWithGranularity`: similar to `AdvanceToNextWake`, with the twist that the
   virtual clock now has a user-specified granularity. The clock always advances in multiples of
   said granularity.

Another difference is that sittard supports sleeps of arbitrary amounts, whereas tokio's minimum sleep duration is of 1 millisecond.
