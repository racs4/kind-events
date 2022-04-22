use std::collections::BTreeSet;
use std::io::ErrorKind;
use std::sync::mpsc::Sender as TSender;
use std::{sync::mpsc::channel, collections::BTreeMap};
use std::thread;
use std::error::Error;

// use websocket::client::ClientBuilder;
// use websocket::{Message, OwnedMessage, CloseData};
use ws::{connect, Message, CloseCode, Handler, Sender, ErrorKind::{Internal}};

use crate::client;
use crate::lib::{self, get_time};

const CONNECTION: &'static str = "ws://127.0.0.1:2794";

pub fn main() {
  let mut client = Client::new(
    CONNECTION, 
    "0x0000000000000000000000000000000000000000000000000000000000000001"
  ).unwrap();

  let room = "1234000000004321";
  let post = "01020304";


  client.send_post(room, post, None).unwrap();
}

#[derive(Clone)]
struct Post {
  tick: String,
  addr: String,
  data: String
}

enum Event {
  Connect(Client),
  Disconnect,
}

#[derive(Clone)]
pub struct Client {
  out: Sender,
  key: String,

  posts: BTreeMap<String, Vec<Post>>,
  watching_rooms: BTreeSet<String>,

  last_ask_time: Option<u64>,
  last_ask_numb: u64,
  best_ask_ping: u64,
  ping: u64,
  delta_time: u64,
  thread_out: TSender<Event>,
}

impl Client {

  fn on_init(&self) {}

  fn on_post(&self) {}

  fn new(url: &'static str, key: &'static str) -> Result<Client, String> {
    let (tx, rx) = channel();

    let thr = thread::spawn(move || {
        connect(url, |sender| Client {
            out: sender,
            thread_out: tx.clone(),
            key: key.to_string(),
            posts: BTreeMap::new(),
            watching_rooms: BTreeSet::new(),
            last_ask_time: None,
            last_ask_numb: 0,
            best_ask_ping: u64::MAX,
            ping: 0,
            delta_time: 0
        }).unwrap();
    });


    let result = if let Ok(Event::Connect(sender)) = rx.recv() {
      Ok(sender)
    } else {
      Err("Could not connect as client".to_string())
    };

    // Ensure the client has a chance to finish up
    thr.join().unwrap();

    result
  }

  fn ask_time(&mut self) {
    self.last_ask_time = Some(lib::get_time());
    self.last_ask_numb += 1;
    let msg = lib::hexs_to_bytes(&[
      lib::u8_to_hex(lib::TIME),
      lib::u64_to_hex(self.last_ask_numb)
    ]).unwrap(); // TODO remove unwrap
    let msg = Message::Binary(msg);
    self.out.send(msg);
  }

  fn send_post(&mut self, post_room: &str, post_data: &str, priv_key: Option<String>) -> Result<(), Box<dyn Error>> {
    let priv_key = priv_key.unwrap_or(self.key.to_string());
    let priv_key = lib::check_hex(256, &priv_key)?;
    let post_room = lib::check_hex(64, post_room)?;
    let post_data = lib::check_hex(0, post_data)?;
    // let post_hash = ethsig.keccak("0x"+lib::hexs_to_bytes([post_room, post_data])).slice(2);
    // let post_sign = ethsig.signMessage("0x"+post_hash, "0x"+priv_key).slice(2);

    let msg_buf = lib::hexs_to_bytes(&[
      lib::u8_to_hex(lib::POST),
      post_room,
      post_data
    ])?;

    self.out.send(msg_buf)?;

    Ok(())
  }

  fn watch_room(&mut self, room_name: &str) {
    let room_name = room_name.to_lowercase();
    if !self.watching_rooms.contains(&room_name) {
      self.watching_rooms.insert(room_name.clone());
      let room_name = lib::check_hex(64, &room_name).unwrap(); // TODO
      let msg_buff = lib::hexs_to_bytes(&[
        lib::u8_to_hex(lib::WATCH),
        room_name.clone()
      ]).unwrap(); // TODO
      self.posts.insert(room_name.clone(), Vec::new());
      self.out.send(msg_buff).unwrap(); // TODO
    }
  }

  fn unwacth_room(&mut self, room_name: &str) {
    let room_name = room_name.to_lowercase();
    if self.watching_rooms.contains(&room_name) {
      self.watching_rooms.remove(&room_name);
      let room_name = lib::check_hex(64, &room_name).unwrap(); // TODO
      let msg_buff = lib::hexs_to_bytes(&[
        lib::u8_to_hex(lib::UNWATCH),
        room_name
      ]).unwrap(); // TODO
      self.out.send(msg_buff).unwrap(); // TODO
    }
  }
}

impl Handler for Client {
  fn on_open(&mut self, shake: ws::Handshake) -> ws::Result<()> {
      self.thread_out
        .send(Event::Connect(self.clone()))
        .map_err(|err| {
            ws::Error::new(
                Internal,
                format!("Unable to communicate between threads: {:?}.", err),
            )
        })
  }

  fn on_message(&mut self, msg: Message) -> ws::Result<()> {
    let msg = msg.into_data();

    if let Some(msg_type) = msg.first() {
      match *msg_type {
        lib::SHOW => {
          let room = lib::bytes_to_hex(&msg[1..9]);
          let tick = lib::bytes_to_hex(&msg[9..17]);
          let addr = lib::bytes_to_hex(&msg[17..37]);
          let data = lib::bytes_to_hex(&msg[37..msg.len()]);

          self.posts.get_mut(&room).unwrap().push(Post{
            tick,
            addr,
            data
          });

          // calls callback
          self.on_post();
        },
        lib::TIME => {
          let map_hex_err = 
            |err| 
              ws::Error::new(Internal, "Could not perform hex conversion");
          let reported_server_time = 
            lib::hex_to_u64(&lib::bytes_to_hex(&msg[1..9]))
              .map_err(map_hex_err)?;
          let reply_numb = 
            lib::hex_to_u64(&lib::bytes_to_hex(&msg[9..17]))
              .map_err(map_hex_err)?;

          if let Some(last_ask_time) = &self.last_ask_time {
            if self.last_ask_numb == reply_numb {
              self.ping = (lib::get_time() - last_ask_time) / 2;
              let local_time = lib::get_time();
              let estimated_server_time = reported_server_time + self.ping;

              if self.ping < self.best_ask_ping {
                self.delta_time = estimated_server_time - local_time;
                self.best_ask_ping = self.ping;
              }
            }
          }
        },
        _ => {}
      }
    }

    Ok(())
  }
}