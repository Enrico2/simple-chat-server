# Simple Chat Server

As an exercise in async stuff in Rust, this is a TCP chat server.

Run:
```shell
cargo run
```

Open a few terminal windows with `nc localhost 7878`

Every text sent from a window will show up in the other windows.

Press enter without sending any text to disconnect. 
