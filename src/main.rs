mod client;
mod lib;
mod server;

use std::env;

#[tokio::main]
async fn main() {
    // let args: Vec<String> = env::args().collect();
    // if let Some(mode) = args.get(1) {
    //     if mode == "--server" {
    //         server::main().unwrap();
    //     } else if mode == "--client" {
    client::main().await;
    //     }
    // }
}
