use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::client_async;
use tungstenite::protocol::Message;

use publichat::server::{run_server, URL};

#[tokio::test]
async fn test_websocket_echo() {
    tokio::spawn(async {
        run_server().await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

    let (ws_stream, _) = client_async(
        "ws://127.0.0.1:7878",
        TcpStream::connect(URL).await.unwrap(),
    )
    .await
    .unwrap();

    let (mut write, mut read) = ws_stream.split();

    let test_message = "hello from test!";
    write
        .send(Message::Text(test_message.into()))
        .await
        .unwrap();

    if let Some(Ok(Message::Text(reply))) = read.next().await {
        assert_eq!(reply, test_message);
    } else {
        panic!("No reply or unexpected message");
    }
}
