use futures_channel::mpsc;
use futures_util::{future, pin_mut, StreamExt};
use log::info;
use log4rs::init_file;
use std::{env, error::Error};
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    spawn,
};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init_file("log4rs_client.yaml", Default::default()).unwrap();
    info!("Initialized the logger");

    let addr = env::args()
        .nth(1)
        .unwrap_or(String::from("ws://127.0.0.1:8080"));
    let url = Url::parse(&addr)?; // addr should start with "ws://" or "wss://"
    let (ws_stream, _) = connect_async(url).await?;
    info!("Connected to {}", addr);

    let (send, receive) = ws_stream.split();

    // send
    let (stdin_tx, stdin_rx) = mpsc::unbounded();
    spawn(read_stdin(stdin_tx));
    let stdin_to_ws = stdin_rx.map(Ok).forward(send);

    // receive
    let ws_to_stdout = {
        receive.for_each(|message| async {
            let data = message.unwrap().into_data();
            io::stdout().write_all(&data).await.unwrap();
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;

    Ok(())
}

// Read data from stdin and send it along the sender provided.
async fn read_stdin(tx: mpsc::UnboundedSender<Message>) {
    let mut stdin = io::stdin();
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
