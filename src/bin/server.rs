use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, StreamExt, TryStreamExt};
use std::{
    collections::HashMap,
    env,
    error::Error,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log4rs::init_file("log4rs_server.yaml", Default::default()).unwrap();
    log::info!("Initialized the logger");

    let addr: String = env::args().nth(1).unwrap_or(String::from("127.0.0.1:8080"));
    let listener: TcpListener = TcpListener::bind(&addr).await?;
    log::info!("Listening on: {}", addr);

    // Spawn the handling of each connection in a separate task.
    let state = PeerMap::new(Mutex::new(HashMap::new()));
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}

async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    log::info!("Incoming TCP connection from: {}", addr);

    // WebSocket handshake
    let ws_stream = tokio_tungstenite::accept_async(raw_stream)
        .await
        .expect("Error during the websocket handshake occurred");
    log::info!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr, tx);

    let (outgoing, incoming) = ws_stream.split();
    let broadcast_incoming = incoming.try_for_each(|msg| {
        log::info!("{}: {}", addr, msg.to_text().unwrap());
        let peers = peer_map.lock().unwrap();
        let peers = peers.clone();

        // Broadcast the message to everyone except ourselves.
        for recp in peers {
            if recp.0 != addr {
                recp.1.unbounded_send(msg.clone()).unwrap();
            }
        }
        future::ok(())
    });

    let receive_from_others = rx.map(Ok).forward(outgoing);

    pin_mut!(broadcast_incoming, receive_from_others);
    future::select(broadcast_incoming, receive_from_others).await;

    log::info!("Disconnected: {}", &addr);
    peer_map.lock().unwrap().remove(&addr);
}
