use std::{
    collections::{HashMap, HashSet},
    future::Future,
};

use futures_util::{SinkExt, StreamExt};

use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use crate::lib;

#[derive(Debug)]
pub enum EventMessage {
    SendPost(String, String, String),
    AskTime(),
    WatchRoom(String),
    UnwatchRoom(String),
}

#[derive(Clone, Debug)]
pub struct Post {
    room: String,
    addr: String,
    tick: String,
    data: String,
}

// Time sync variables
struct TimeSync {
    pub last_ask_time: Option<u64>,
    pub last_ask_numb: u64,
    pub best_ask_ping: u64,
    pub delta_time: u64,
    pub ping: u64,
}

impl TimeSync {
    fn new() -> TimeSync {
        TimeSync {
            last_ask_time: None::<u64>, // last time we pinged the server
            last_ask_numb: 0_u64,       // id of the last ask request
            best_ask_ping: u64::MAX,    // best ping we got
            delta_time: 0_u64,          // estimated time on best ping
            ping: 0_u64,                // current ping
        }
    }
}

pub type PostsMap = HashMap<String, Vec<Post>>;
pub type OnInitFn = Box<dyn FnMut()>;
pub type OnPostFn = Box<dyn FnMut(&Post, &mut Vec<Post>) + Send + 'static>;
pub type Callback = Box<dyn FnMut(&mut Client) + Send + 'static>;

pub async fn main(mut on_init: OnInitFn, mut on_post: OnPostFn, mut callback: Callback) {
    let connect_addr = "ws://127.0.0.1:7171";
    let url = url::Url::parse(connect_addr).unwrap();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<EventMessage>();
    let mut rx = UnboundedReceiverStream::new(rx);
    let (mut ws_stream, _) = connect_async(url).await.expect("Failed to connect");

    // Posts store
    let mut posts = HashMap::new();
    let mut time_sync = TimeSync::new();

    on_init();
    let ws_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                received = ws_stream.next() => {
                    if let Some(received) = received {
                        match received {
                            Ok(received) => {
                                match received {
                                    Message::Binary(data) => {
                                        on_message(&data, &mut posts, &mut on_post, &mut time_sync);
                                    },
                                    _ => {}
                                }
                            },
                            Err(err) => {
                                println!("{}", err.to_string());
                            }
                        }
                    }
                },
                to_send = rx.next() => {
                    if let Some(to_send) = to_send {
                        match to_send {
                            EventMessage::SendPost(room, data, key) => {
                                let _key = lib::check_hex(256, &key).unwrap();
                                let room = lib::check_hex(64, &room).unwrap();
                                let data = lib::check_hex(0, &data).unwrap();
                                // let post_hash = ethsig.keccak("0x"+lib::hexs_to_bytes([post_room, data])).slice(2);
                                // let post_sign = ethsig.signMessage("0x"+post_hash, "0x"+key).slice(2);
                                let buff = vec![
                                    lib::u8_to_hex(lib::POST),
                                    room,
                                    data,
                                ];
                                let buff = lib::hexs_to_bytes(&buff).map_err(|err| err.to_string()).unwrap();
                                ws_stream.send(Message::Binary(buff)).await.unwrap();
                            },
                            EventMessage::WatchRoom(room) => {
                                let room = lib::check_hex(64, &room).unwrap();
                                let buff = vec![lib::u8_to_hex(lib::WATCH), room];
                                let buff = lib::hexs_to_bytes(&buff).unwrap();
                                ws_stream.send(Message::Binary(buff)).await.unwrap();
                            },
                            EventMessage::UnwatchRoom(room) => {
                                let room = lib::check_hex(64, &room).unwrap();
                                let buff = vec![lib::u8_to_hex(lib::UNWATCH), room];
                                let buff = lib::hexs_to_bytes(&buff).unwrap();
                                ws_stream.send(Message::Binary(buff)).await.unwrap();
                            }
                            EventMessage::AskTime() => {
                                time_sync.last_ask_time = Some(lib::get_time());
                                time_sync.last_ask_numb += 1;
                                let buff = vec![
                                    lib::u8_to_hex(lib::TIME),
                                    lib::u64_to_hex(time_sync.last_ask_numb),
                                  ];
                                let buff = lib::hexs_to_bytes(&buff).unwrap();
                                ws_stream.send(Message::Binary(buff)).await.unwrap();
                            }
                        }
                    }
                }
            }
        }
    });

    let client_task = tokio::spawn(async move {
        let mut client = Client::new(tx.clone(), "12344321");
        callback(&mut client);
    });

    tokio::join!(ws_task, client_task);
}

pub struct Client {
    key: String,
    tx: UnboundedSender<EventMessage>,
    watching: HashSet<String>,
}

impl Client {
    pub fn new(tx: UnboundedSender<EventMessage>, key: &str) -> Client {
        Client {
            watching: HashSet::new(),
            key: key.to_string(),
            tx,
        }
    }

    // Sends a signed post to a room on the server
    pub fn send_post(&self, room: &str, data: &str, priv_key: Option<&str>) {
        let key = priv_key.unwrap_or(self.key.as_str());
        self.tx
            .send(EventMessage::SendPost(
                room.to_string(),
                data.to_string(),
                key.to_string(),
            ))
            .unwrap();
    }

    pub fn watch_room(&mut self, room: &str) {
        let room = room.to_lowercase();
        if !self.watching.contains(&room) {
            self.watching.insert(room.to_string());
            self.tx
                .send(EventMessage::WatchRoom(room))
                .unwrap();
        }
    }

    pub fn unwatch_room(&mut self, room: &str) {
        let room = room.to_lowercase();
        if self.watching.contains(&room) {
            self.watching.remove(&room);
            self.tx
                .send(EventMessage::UnwatchRoom(room.to_string()))
                .unwrap();
        }
    }

    pub fn ask_time(&self) {
        self.tx.send(EventMessage::AskTime()).unwrap();
    }
}

fn on_message(
    data: &[u8],
    posts: &mut PostsMap,
    on_post: &mut OnPostFn,
    time_sync: &mut TimeSync,
) {
    if let Some(msg_type) = data.first() {
        match *msg_type {
            lib::SHOW => {
                // dbg!(data);
                let room = lib::bytes_to_hex(&data[1..9]);
                let tick = lib::bytes_to_hex(&data[9..17]);
                let addr = lib::bytes_to_hex(&data[17..37]);
                let data = lib::bytes_to_hex(&data[37..data.len()]);

                let posts_vec = posts.get_mut(&room);
                let post = Post {
                    room: room.clone(),
                    tick,
                    addr,
                    data,
                };
                match posts_vec {
                    None => {
                        posts.insert(room.clone(), vec![post.clone()]);
                    }
                    Some(posts_vec) => posts_vec.push(post.clone()),
                }
                let posts = posts.get_mut(&room).unwrap();
                (on_post)(&post, posts);
            }
            lib::TIME => {
                let reported_server_time =
                    lib::hex_to_u64(&lib::bytes_to_hex(&data[1..9])).unwrap();
                let reply_numb = lib::hex_to_u64(&lib::bytes_to_hex(&data[9..17])).unwrap();
                if time_sync.last_ask_time.is_some() && time_sync.last_ask_numb == reply_numb {
                    let last_ask_time = time_sync.last_ask_time.unwrap();
                    time_sync.ping = (lib::get_time() - last_ask_time) / 2;
                    let local_time = lib::get_time();
                    let estimated_server_time = reported_server_time + time_sync.ping;
                    if time_sync.ping < time_sync.best_ask_ping {
                        time_sync.delta_time = estimated_server_time - local_time;
                        time_sync.best_ask_ping = time_sync.ping;
                    }
                }
            }
            _ => {}
        }
    }
}
