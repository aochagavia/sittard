use sittard::rt::Rt;
use std::time::Duration;

fn main() {
    let rt = Rt::default();
    rt.block_on(async {
        rt.spawn(async move {
            sittard::time::sleep(Duration::from_millis(200)).await;
            println!("200ms later");
        });

        rt.spawn(async move {
            sittard::time::sleep(Duration::from_millis(100)).await;
            println!("100ms later");
        });

        rt.spawn(async move {
            sittard::time::sleep(Duration::from_millis(300)).await;
            println!("300ms later");
        });

        // Wait for all tasks to complete
        sittard::time::sleep(Duration::from_secs(1)).await;
    });
}
