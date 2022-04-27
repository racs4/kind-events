// use std::collections::BTreeMap;
// use std::error::Error;
// use std::fmt::format;
// use std::fs::read_to_string;
// use std::io::Write;
// use std::net::TcpStream;
// use std::time::{SystemTime, UNIX_EPOCH};
// use std::{fs, io};
// use std::path::Path;
// use std::thread;
// // use websocket::header::Connection;
// // use websocket::sync::{Server, Writer};
// // use websocket::ws::Sender;
// // use websocket::{OwnedMessage, Message};
// use ws::{listen, Handler, Sender, Message};

// use crate::lib;

// type RoomID = String;

// #[derive(Clone)]
// struct Connections {
//   room_posts: BTreeMap<RoomID, Vec<Vec<u8>>>,
//   watch_list: BTreeMap<RoomID, Vec<u64>>,
// }

// impl Connections {
//   fn watch_room(&mut self, room_name: &str, ws: u64) {
//     // Creates watcher list
//     if self.watch_list.get(room_name).is_none() {
//       self.watch_list.insert(room_name.to_string(), Vec::new());
//     }

//     // Gets watcher list
//     let watchlist = self.watch_list.get_mut(room_name).unwrap();

//     // Makes sure user isn't watching already
//     for watcher in watchlist.to_owned() { // TODO falar com kelvin
//       if watcher == ws { return; }
//     }

//     // Sends old messages
//     if let Some(posts) = self.room_posts.get(room_name) {
//       for post in posts {
//         // TODO websocket
//       }
//     }

//     // Adds user to watcher list
//     watchlist.push(ws);
//   }

//   fn unwacth_room(&mut self, room_name: &str, ws: u64) {
//     let mut empty = vec![];
//     // Gets watcher list
//     let watchlist = self.watch_list.get_mut(room_name).unwrap_or(&mut empty);

//     // Removes user from watcher list
//     for (i, watcher) in watchlist.iter().enumerate() {
//       if *watcher == ws {
//         for j in i..watchlist.len() {
//           watchlist[j] = watchlist[j + 1];
//         };
//         return;
//       }
//     }
//   }

//   // Saves a post (room id, user address, data)
//   fn save_post(&mut self, post_room: &str, post_user: &str, post_data: &str, dir_path: &Path) -> Result<(), Box<dyn Error>> {
//     let post_room = lib::check_hex(64, post_room)?;
//     let post_tick = lib::u64_to_hex(get_tick());
//     let post_user = lib::check_hex(160, post_user)?;
//     let post_data = lib::check_hex(0, post_data)?;
//     let post_list = vec![post_room.clone(), post_tick, post_user.clone(), post_data.clone()];
//     let mut post_buf_hex = vec![lib::u8_to_hex(lib::SHOW)];
//     post_buf_hex.extend(post_list.clone());
//     println!("rapaaaz");
//     println!("{:?}", post_buf_hex);
//     let post_buff = lib::hexs_to_bytes(&post_buf_hex)?;
//     println!("rapaaaz");
//     let mut post_seri_hex = vec![lib::u32_to_hex((post_buff.len()-1) as u32)];
//     post_seri_hex.extend(post_list.clone());
//     let post_seri = lib::hexs_to_bytes(&post_seri_hex)?;
//     let post_file = dir_path.join(format!("{}.room", post_room));
//     println!("rapaaaz");
//     let mut log_msg = format!("Saving post!
//     - post_room: {}
//     - post_user: {}
//     - post_data: {}
//     - post_file: {}.room
//   ", &post_room, &post_user, &post_data, &post_room);

//     // Creates reconnection array for this room
//     if self.room_posts.get(&post_room).is_none() {
//       self.room_posts.insert(post_room.clone(), Vec::new());
//     }

//     // Adds post to reconnection array
//     self.room_posts.get_mut(&post_room).unwrap().push(post_buff);
//     println!("rapaaaz");
//     // Broadcasts
//     if let Some(watchers) = self.watch_list.get(&post_room) {
//       log_msg = format!("{}\n - broadcasting to {} watcher(s).\n", log_msg, watchers.len());
//       for ws in watchers {
//         //TODO websocket
//       }
//     }

//     // Create file for this room
//     let mut file =
//     if !post_file.exists() {
//       fs::OpenOptions::new()
//         .create_new(true)
//         .write(true)
//         .append(true)
//         .open(post_file)?
//     } else {
//       fs::OpenOptions::new()
//         .write(true)
//         .append(true)
//         .open(post_file)?
//     };

//     // Adds post to file
//     file.write_all(&post_seri);

//     // Log messages
//     println!("{}", log_msg);

//     Ok(())
//   }

//   fn new() -> Connections {
//     Connections {
//       room_posts: BTreeMap::new(),
//       watch_list: BTreeMap::new()
//     }
//   }
// }

// pub fn main() -> Result<(), Box<dyn Error>> {
//   let mut conn = Connections::new();
//   let connected: usize = 0;

//   // Creates the data directory
//   let dir_path = Path::new("./data");
//   if !dir_path.exists() {
//     fs::create_dir(dir_path)?;
//   }

//   // Load existing posts
//   let files = fs::read_dir(dir_path)?;
//   for file in files {
//     let file = file?;
//     let file_name = file.file_name();
//     let file_name = file_name.to_str().unwrap(); // TODO

//     if file_name.ends_with(".room") {
//       let room_name = &file_name[0..file_name.len() - 5];
//       let file_data = fs::read(dir_path.join(file_name))?;

//       let mut room_posts = Vec::new();
//       let mut i = 0;
//       while i < file_data.len() {
//         let hex = lib::bytes_to_hex(&file_data[i..i+4]);
//         let size = lib::hex_to_u32(&hex)? as usize;
//         let mut head = Vec::from([lib::SHOW]); // TODO verifiy
//         let mut body = Vec::from(&file_data[i+4..i+4+size]);
//         head.append(&mut body);
//         room_posts.push(head);
//         i += 4 + size;
//       }

//       println!("Loaded {} posts on room {}.", room_posts.len(), room_name);
//       conn.room_posts.insert(room_name.to_string(), room_posts);
//     }
//   }

//   listen("127.0.0.1:2794", |sender| Server {
//     out: sender,
//     conn: conn.clone(),
//     dir_path
//   });

//   Ok(())
// }

// // Server WebSocket handler
// struct Server<'a> {
//   out: Sender,
//   conn: Connections,
//   dir_path: &'a Path,
// }

// impl <'a> Handler for Server<'a> {
//   fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
//       match msg {
//         ws::Message::Text(msg) => {
//           let data = lib::string_to_bytes(&msg);
//           on_message(&mut self.conn, &data, &self.dir_path, &self.out);
//         },
//         ws::Message::Binary(msg) => {
//           let data = msg;
//           on_message(&mut self.conn, &data, &self.dir_path, &self.out);
//         },
//     }

//     Ok(())
//   }
// }

// // Methods
// // =======

// // Returns current time
// fn get_time() -> u64 {
//   let start = SystemTime::now();
//   let since_the_epoch = start
//         .duration_since(UNIX_EPOCH)
//         .expect("Time went backwards");
//   since_the_epoch.as_millis() as u64
// }

// // Returns current tick
// fn get_tick() -> u64 {
//   (get_time() as f64 / 62.5).floor() as u64
// }

// fn on_message(conn: &mut Connections, data: &[u8], dir_path: &Path, sender: &Sender) -> Result<(), Box<dyn Error>> {
//   if let Some(c) = data.first() {
//     match *c {
//       // user wants to watch a room
//       lib::WATCH => {
//         let room = lib::bytes_to_hex(&data[1..9]);
//         conn.watch_room(&room, 0)
//       }
//       lib::UNWATCH => {
//         let room = lib::bytes_to_hex(&data[1..9]);
//         conn.unwacth_room(&room, 0)
//       }
//       lib::TIME => {
//         let msge_buff = lib::hexs_to_bytes(&[
//           lib::u8_to_hex(lib::TIME),
//           lib::u64_to_hex(get_time()),
//           lib::bytes_to_hex(&data[1..9])
//         ])?;
//         let msge_buff = Message::binary(msge_buff);
//         sender.send(msge_buff);
//       }
//       lib::POST => {
//         println!("HERE PORRA");
//         let post_room = lib::bytes_to_hex(&data[1..9]);
//         let post_data = lib::bytes_to_hex(&data[9..data.len() - 65]);
//         let post_sign = lib::bytes_to_hex(&data[data.len() - 65..data.len()]);
//         // let post_hash = ethsig.keccak("0x"+lib::hexs_to_bytes([post_room, post_data])).slice(2);
//         // let post_user = ethsig.signerAddress("0x"+post_hash, "0x"+post_sign).slice(2);
//         let post_user = lib::string_to_hex("Vasco"); // TODO
//         conn.save_post(&post_room, &post_user, &post_data, dir_path);
//       }
//       _ => {}
//     }
//   }

//   Ok(())
// }
