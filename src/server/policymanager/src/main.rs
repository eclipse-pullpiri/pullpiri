pub mod grpc;

fn main() {
    println!("Hello, world!");
    loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
