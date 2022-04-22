mod lib;
mod server;
mod client;

use std::env;

fn main() {
    // let args: Vec<String> = env::args().collect();
    // if let Some(mode) = args.get(1) {
    //     if mode == "--server" {
    //         server::main().unwrap();
    //     } else if mode == "--client" {
            client::main();
    //     }
    // }
}
