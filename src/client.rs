//! A simple example of hooking up stdin/stdout to a WebSocket stream.
//!
//! This example will connect to a server specified in the argument list and
//! then forward all data read on stdin to the server, printing out all data
//! received on stdout.
//!
//! Note that this is not currently optimized for performance, especially around
//! buffer management. Rather it's intended to show an example of working with a
//! client.
//!
//! You can use this example together with the `server` example.

use std::{
    collections::{BTreeMap, BTreeSet},
    env, ops::Deref,
};

use crate::{client, lib};
use futures_util::{future, pin_mut, StreamExt, SinkExt, Future};
use std::error::Error;
use tokio::{io::{self, AsyncReadExt, AsyncWriteExt}, net::TcpStream};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream, MaybeTlsStream};
pub async fn main() {
    
}

pub async fn api<Fut: 'static + Future<Output = ()> + std::marker::Send>(url: &str, key: &str, mut f: impl FnMut(&mut Client) -> Fut) {
    let connect_addr = env::args()
        .nth(1)
        .unwrap_or(url.to_string());
    let url = url::Url::parse(&connect_addr).unwrap();

    // let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
    

    let (mut ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let mut client = Client::new(key.to_string(), Box::new(ws_stream), Box::new(|| {}), Box::new(|| {}));
    tokio::spawn(f(&mut client));


    // let stdin_to_ws = stdin_rx.map(Ok).forward(write);
    let ws_to_stdout = async {
        while let Some(msg) = ws_stream.next().await {
            let msg = msg.expect("not valid");
            if msg.is_text() || msg.is_binary() {
                println!("aqui");
            }
        }
    };
    ws_to_stdout.await;
    // pin_mut!(stdin_to_ws, ws_to_stdout);
    // future::select(stdin_to_ws, ws_to_stdout).await;
}

async fn example(client: &mut Client) {
    client.send_post("ffff", "ffff", None);
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx.unbounded_send(Message::binary(buf)).unwrap();
    }
}

// #[derive(Clone)]
struct Post {
    tick: String,
    addr: String,
    data: String,
}

// enum Event {
//   Connect(Client),
//   Disconnect,
// }

type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub struct Client {
    ws: Box<Ws>,
    key: String,
    posts: BTreeMap<String, Vec<Post>>,
    watching_rooms: BTreeSet<String>,
    last_ask_time: Option<u64>,
    last_ask_numb: u64,
    best_ask_ping: u64,
    ping: u64,
    delta_time: u64,
    on_init: Box<dyn FnOnce()>,
    on_post: Box<dyn FnOnce()>,
}

impl Client {
    fn new(key: String, ws: Box<Ws>, on_init: Box<dyn FnOnce()>, on_post: Box<dyn FnOnce()>) -> Client {
        Client {
            ws,
            key,
            posts: BTreeMap::new(),
            watching_rooms: BTreeSet::new(),
            last_ask_time: None,
            last_ask_numb: 0,
            best_ask_ping: u64::MAX,
            ping: 0,
            delta_time: 0,
            on_init,
            on_post,
        }
    }

    async fn ask_time(&mut self) -> Result<(), Box<dyn Error>> {
        self.last_ask_time = Some(lib::get_time());
        self.last_ask_numb += 1;
        let msg = lib::hexs_to_bytes(&[
            lib::u8_to_hex(lib::TIME),
            lib::u64_to_hex(self.last_ask_numb),
        ])
        .unwrap(); // TODO remove unwrap
        let msg = Message::Binary(msg);
        self.ws.send(msg).await?;
        Ok(())
    }

    async fn send_post(
        &mut self,
        post_room: &str,
        post_data: &str,
        priv_key: Option<String>,
    ) -> Result<(), Box<dyn Error>> {
        let priv_key = priv_key.unwrap_or(self.key.to_string());
        let priv_key = lib::check_hex(256, &priv_key)?;
        let post_room = lib::check_hex(64, post_room)?;
        let post_data = lib::check_hex(0, post_data)?;
        // let post_hash = ethsig.keccak("0x"+lib::hexs_to_bytes([post_room, post_data])).slice(2);
        // let post_sign = ethsig.signMessage("0x"+post_hash, "0x"+priv_key).slice(2);

        let msg = lib::hexs_to_bytes(&[lib::u8_to_hex(lib::POST), post_room, post_data])?;
        let msg = Message::Binary(msg);

        self.ws.send(msg).await?;

        Ok(())
    }

    fn watch_room(&mut self, room_name: &str) {
        let room_name = room_name.to_lowercase();
        if !self.watching_rooms.contains(&room_name) {
            self.watching_rooms.insert(room_name.clone());
            let room_name = lib::check_hex(64, &room_name).unwrap(); // TODO
            let msg_buff =
                lib::hexs_to_bytes(&[lib::u8_to_hex(lib::WATCH), room_name.clone()]).unwrap(); // TODO
            self.posts.insert(room_name.clone(), Vec::new());
            //   self.out.send(msg_buff).unwrap(); // TODO
        }
    }

    fn unwacth_room(&mut self, room_name: &str) {
        let room_name = room_name.to_lowercase();
        if self.watching_rooms.contains(&room_name) {
            self.watching_rooms.remove(&room_name);
            let room_name = lib::check_hex(64, &room_name).unwrap(); // TODO
            let msg_buff = lib::hexs_to_bytes(&[lib::u8_to_hex(lib::UNWATCH), room_name]).unwrap();
            // TODO
            //   self.out.send(msg_buff).unwrap(); // TODO
        }
    }
}

// impl Handler for Client {
//   fn on_open(&mut self, shake: ws::Handshake) -> ws::Result<()> {
//       self.thread_out
//         .send(Event::Connect(self.clone()))
//         .map_err(|err| {
//             ws::Error::new(
//                 Internal,
//                 format!("Unable to communicate between threads: {:?}.", err),
//             )
//         })
//   }

//   fn on_message(&mut self, msg: Message) -> ws::Result<()> {
//     let msg = msg.into_data();

//     if let Some(msg_type) = msg.first() {
//       match *msg_type {
//         lib::SHOW => {
//           let room = lib::bytes_to_hex(&msg[1..9]);
//           let tick = lib::bytes_to_hex(&msg[9..17]);
//           let addr = lib::bytes_to_hex(&msg[17..37]);
//           let data = lib::bytes_to_hex(&msg[37..msg.len()]);

//           self.posts.get_mut(&room).unwrap().push(Post{
//             tick,
//             addr,
//             data
//           });

//           // calls callback
//           self.on_post();
//         },
//         lib::TIME => {
//           let map_hex_err =
//             |err|
//               ws::Error::new(Internal, "Could not perform hex conversion");
//           let reported_server_time =
//             lib::hex_to_u64(&lib::bytes_to_hex(&msg[1..9]))
//               .map_err(map_hex_err)?;
//           let reply_numb =
//             lib::hex_to_u64(&lib::bytes_to_hex(&msg[9..17]))
//               .map_err(map_hex_err)?;

//           if let Some(last_ask_time) = &self.last_ask_time {
//             if self.last_ask_numb == reply_numb {
//               self.ping = (lib::get_time() - last_ask_time) / 2;
//               let local_time = lib::get_time();
//               let estimated_server_time = reported_server_time + self.ping;

//               if self.ping < self.best_ask_ping {
//                 self.delta_time = estimated_server_time - local_time;
//                 self.best_ask_ping = self.ping;
//               }
//             }
//           }
//         },
//         _ => {}
//       }
//     }

//     Ok(())
//   }
// }
