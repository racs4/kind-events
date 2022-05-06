use std::{
    collections::HashMap,
    error::Error,
    fs,
    io::{Error as IoError, Write},
    net::SocketAddr,
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::lib;
use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;

type Socket = (SocketAddr, UnboundedSender<Message>);

async fn handle_connection(raw_stream: TcpStream, addr: SocketAddr, conn: Arc<Mutex<Connections>>) {
    // Accept connection
    println!("Incoming TCP connection from: {}", addr);
    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    println!("WebSocket connection established: {}", addr);

    // Create channel to communicate between threads
    let (tx, rx) = unbounded();
    // Split ws_stream in reader and writer
    let (outgoing, incoming) = ws_stream.split();

    // Message receiving handler
    let receive_handler = incoming.try_for_each(|msg| {
        println!("Received a message from {}: {:?}", addr, msg.to_text());
        match msg {
            Message::Binary(ref data) => {
                on_message(&mut conn.lock().unwrap(), data, addr, tx.clone()).unwrap();
            }
            Message::Text(ref data) => {
                let data = data.as_bytes();
                on_message(&mut conn.lock().unwrap(), data, addr, tx.clone()).unwrap();
            }
            _ => {}
        }
        future::ok(())
    });

    // Message sender handler
    // Receives Message from tx senders and sends then
    let sender_handler = rx.map(Ok).forward(outgoing);

    // Receives and sends Messages concurrently
    pin_mut!(receive_handler, sender_handler);
    future::select(receive_handler, sender_handler).await;

    // Removes socket from all rooms
    // TODO: Remove unwrap?
    conn.lock()
        .unwrap()
        .watch_list
        .iter_mut()
        .for_each(|(_room, sockets)| sockets.retain(|(socket_addr, _)| *socket_addr != addr));

    println!("{} disconnected", &addr);
}

pub async fn main() -> Result<(), IoError> {
    let addr = "127.0.0.1:7171".to_string();

    // Create connections struct
    // This will store room posts and rooms watch list between threads
    let conn = Arc::new(Mutex::new(Connections::new()));

    // Creates the data directory
    let dir_path = Path::new("./data");
    if !dir_path.exists() {
        fs::create_dir(dir_path)?;
    }

    // Load existing posts
    let files = fs::read_dir(Path::new("./data"))?;
    for file in files {
        let file = file?;
        let file_name = file.file_name();
        let file_name = file_name.to_str().unwrap(); // TODO

        if file_name.ends_with(".room") {
            let room_name = &file_name[0..file_name.len() - 5];
            let file_data = fs::read(Path::new("./data").join(file_name))?;

            let mut room_posts_vec = Vec::new();
            let mut i = 0;
            while i < file_data.len() {
                let hex = lib::bytes_to_hex(&file_data[i..i + 4]);
                let size = lib::hex_to_u32(&hex).unwrap() as usize;
                let mut head = Vec::from([lib::SHOW]); // TODO verifiy
                let mut body = Vec::from(&file_data[i + 4..i + 4 + size]);
                head.append(&mut body);
                room_posts_vec.push(head);
                i += 4 + size;
            }

            println!(
                "Loaded {} posts on room {}.",
                room_posts_vec.len(),
                room_name
            );

            // Put then in the connection Store
            conn.lock()
                .unwrap()
                .room_posts
                .insert(room_name.to_string(), room_posts_vec);
        }
    }

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(&addr).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}", addr);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(stream, addr, conn.clone()));
    }

    Ok(())
}

// Store the connections and posts for each room
#[derive(Clone)]
struct Connections {
    room_posts: HashMap<String, Vec<Vec<u8>>>,
    watch_list: HashMap<String, Vec<Socket>>,
}

impl Connections {
    // Adds socket in watch list and send stored posts to the new socket
    fn watch_room(&mut self, room_name: &str, socket: &Socket) {
        let (socket_addr, socket_tx) = socket;

        // Creates watcher list
        if self.watch_list.get(room_name).is_none() {
            self.watch_list.insert(room_name.to_string(), Vec::new());
        }

        // Gets watcher list
        let watchlist = self.watch_list.get_mut(room_name).unwrap();

        // Makes sure user isn't watching already
        for (user_addr, _) in watchlist.to_owned() {
            // TODO falar com kelvin
            if user_addr == *socket_addr {
                return;
            }
        }

        // Sends old messages
        if let Some(posts) = self.room_posts.get(room_name) {
            for post in posts {
                socket_tx
                    .unbounded_send(Message::Binary(post.to_owned()))
                    .unwrap();
            }
        }

        // Adds user to watcher list
        watchlist.push(socket.clone());
    }

    fn unwacth_room(&mut self, room_name: &str, socket: &Socket) {
        let (socket_addr, _socket_tx) = socket;
        let mut empty = vec![];
        // Gets watcher list
        let watchers = self.watch_list.get_mut(room_name).unwrap_or(&mut empty);

        // Removes user from watcher list
        for (i, (watcher_addr, _)) in watchers.iter().enumerate() {
            if *watcher_addr == *socket_addr {
                watchers.remove(i);
                return;
            }
        }
    }

    // Saves a post (room id, user address, data)
    fn save_post(
        &mut self,
        post_room: &str,
        post_user: &str,
        post_data: &str,
    ) -> Result<(), Box<dyn Error>> {
        let post_room = lib::check_hex(64, post_room)?;
        let post_tick = lib::u64_to_hex(get_tick());
        let post_user = lib::check_hex(160, post_user)?;
        let post_data = lib::check_hex(0, post_data)?;
        let post_list = vec![
            post_room.clone(),
            post_tick,
            post_user.clone(),
            post_data.clone(),
        ];
        let mut post_buf_hex = vec![lib::u8_to_hex(lib::SHOW)];
        post_buf_hex.extend(post_list.clone());
        let post_buff = lib::hexs_to_bytes(&post_buf_hex)?;
        let mut post_seri_hex = vec![lib::u32_to_hex((post_buff.len() - 1) as u32)];
        post_seri_hex.extend(post_list);
        let post_seri = lib::hexs_to_bytes(&post_seri_hex)?;
        let post_file = Path::new("./data").join(format!("{}.room", post_room));
        let mut log_msg = format!(
            "Saving post!
    - post_room: {}
    - post_user: {}
    - post_data: {}
    - post_file: {}.room
  ",
            &post_room, &post_user, &post_data, &post_room
        );

        // Creates reconnection array for this room
        if self.room_posts.get(&post_room).is_none() {
            self.room_posts.insert(post_room.clone(), Vec::new());
        }

        // Adds post to reconnection array
        self.room_posts
            .get_mut(&post_room)
            .unwrap()
            .push(post_buff.clone());

        // Broadcasts
        if let Some(watchers) = self.watch_list.get(&post_room) {
            log_msg = format!(
                "{}\n - broadcasting to {} watcher(s).\n",
                log_msg,
                watchers.len()
            );
            for (_addr, tx) in watchers {
                tx.unbounded_send(Message::Binary(post_buff.clone()))
                    .unwrap();
            }
        }

        // Create file for this room if not exist
        let mut file = if !post_file.exists() {
            fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .append(true)
                .open(post_file)?
        } else {
            fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(post_file)?
        };

        // Adds post to file
        file.write_all(&post_seri)?;

        // Log messages
        println!("{}", log_msg);

        Ok(())
    }

    fn new() -> Connections {
        Connections {
            room_posts: HashMap::new(),
            watch_list: HashMap::new(),
        }
    }
}

// Returns current time
fn get_time() -> u64 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    since_the_epoch.as_millis() as u64
}

// Returns current tick
fn get_tick() -> u64 {
    (get_time() as f64 / 62.5).floor() as u64
}

fn on_message(
    conn: &mut Connections,
    data: &[u8],
    addr: SocketAddr,
    tx: UnboundedSender<Message>,
) -> Result<(), Box<dyn Error>> {
    if let Some(c) = data.first() {
        match *c {
            // user wants to watch a room
            lib::WATCH => {
                let room = lib::bytes_to_hex(&data[1..9]);
                conn.watch_room(&room, &(addr, tx))
            }
            lib::UNWATCH => {
                let room = lib::bytes_to_hex(&data[1..9]);
                conn.unwacth_room(&room, &(addr, tx))
            }
            lib::TIME => {
                let msge_buff = lib::hexs_to_bytes(&[
                    lib::u8_to_hex(lib::TIME),
                    lib::u64_to_hex(get_time()),
                    lib::bytes_to_hex(&data[1..9]),
                ])?;
                let msge_buff = Message::binary(msge_buff);
                tx.unbounded_send(msge_buff).unwrap();
            }
            lib::POST => {
                let post_room = lib::bytes_to_hex(&data[1..9]);
                let post_data = lib::bytes_to_hex(&data[9..data.len()]);
                // TODO: add eth signer
                // let post_sign = lib::bytes_to_hex(&data[data.len() - 65..data.len()]);
                // let post_hash = ethsig.keccak("0x"+lib::hexs_to_bytes([post_room, post_data])).slice(2);
                // let post_user = ethsig.signerAddress("0x"+post_hash, "0x"+post_sign).slice(2);
                let post_user = lib::string_to_hex("Vasco"); // TODO
                conn.save_post(&post_room, &post_user, &post_data).unwrap();
            }
            _ => {}
        }
    }

    Ok(())
}
