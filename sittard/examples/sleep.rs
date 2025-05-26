use sittard::Runtime;
use std::time::Duration;

fn main() {
    // Create a runtime and run a future
    let rt = Runtime::default();
    rt.block_on(async move {
        let now = sittard::time::Instant::now();
        sittard::time::sleep(Duration::from_secs(60)).await;
        let elapsed_secs = now.elapsed().as_secs_f64();
        println!("Here we are, {elapsed_secs} seconds later!");
    });
}
