use std::env;

use client::Client;

mod client;
mod lib;
mod server;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if let Some(mode) = args.get(1) {
        if mode == "--server" {
            // Run server
            server::main().await.unwrap();
        } else if mode == "--client" {
            // Run client example
            let mut counter = 1;

            let callback = |client: &mut Client| {
                client.watch_room("1234000000004321");

                let room = "1234000000004321";
                let post = "01020304";

                client.send_post(room, post, None)
            };

            client::main(
                Box::new(|| println!("Iniciado")),
                Box::new(move |post, posts| {
                    println!("{:?}", post);
                    println!("{}", posts.len());
                    counter += 1;
                }),
                Box::new(callback),
            )
            .await;
        }
    }
}
