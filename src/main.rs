use std::io;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878").await?;
    let stored_sockets: Vec<(OwnedWriteHalf, u16)> = vec![];
    let sockets = Arc::new(Mutex::new(stored_sockets));
    let (sender, mut receiver) = mpsc::unbounded_channel::<(u16, String)>();
    let sender = Arc::new(sender);
    let stop_byte: u8 = 10;

    let receiver_sockets = sockets.clone();
    tokio::spawn(async move {
        while let Some((ref sender_id, msg)) = receiver.recv().await {
            if msg.as_bytes().get(0).unwrap() == &stop_byte {
                disconnect_socket(&receiver_sockets, sender_id).await
            } else {
                fanout(&receiver_sockets, sender_id, msg).await
            }
        }
    });

    let mut socket_id: u16 = 0;
    loop {
        let (socket, addr) = listener.accept().await?;
        println!("new client: {:?} (id: {})", addr, socket_id);

        let (mut r, w) = socket.into_split();
        let mut sockets = sockets.lock().await;
        (*sockets).push((w, socket_id));
        drop(sockets);

        let sender = sender.clone();

        tokio::spawn(async move {
            let mut buf: Vec<u8> = vec![0; 1024];
            loop {
                match r.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(_) => {
                        sender
                            .send((socket_id, String::from(std::str::from_utf8(&buf).unwrap())))
                            .unwrap();
                        reset_buf(&mut buf)
                    }
                    Err(e) => {
                        // Unexpected socket error. There isn't much we can do
                        // here so just stop processing.
                        eprintln!("{:?}", e);
                        break;
                    }
                }
            }
        });
        socket_id += 1;
    }
}

async fn fanout(
    receiver_sockets: &Arc<Mutex<Vec<(OwnedWriteHalf, u16)>>>,
    sender_id: &u16,
    msg: String,
) {
    for (ref mut socket_writer, id) in (*receiver_sockets.lock().await).iter_mut() {
        if id != sender_id {
            let _ = socket_writer.write(msg.as_bytes()).await.unwrap();
        }
    }
}

async fn disconnect_socket(
    receiver_sockets: &Arc<Mutex<Vec<(OwnedWriteHalf, u16)>>>,
    sender_id: &u16,
) {
    println!("disconnecting socket id: {}", sender_id);
    let sockets = &mut (*receiver_sockets.lock().await);
    for (idx, (_, id)) in sockets.iter().enumerate() {
        if id == sender_id {
            // drops the writer so the socket is disconnected.
            sockets.remove(idx);
            break;
        }
    }
}

fn reset_buf(buf: &mut Vec<u8>) {
    for v in buf.iter_mut() {
        *v = 0;
    }
}
