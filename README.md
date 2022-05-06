#### To execute server:

```
  cargo r -- --server
```

#### To execute client example:

```
  cargo r -- --client
```

The client example code is in `src/main.rs`:

```rust
let callback = |client: &mut Client| {
    let room = "1234000000004321";
    let room = "1234000000004321";

    client.watch_room(room);
    client.send_post(room, post, None)
};

client::main(
    Box::new(|| println!("Iniciado")), // on_init
    Box::new(move |post, posts| {      // on_post
        println!("{:?}", post);
        println!("{}", posts.len());
    }),
    Box::new(callback),                // what to do with client
)
```
