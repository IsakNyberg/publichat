use std::path::Path;

use futures::StreamExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;

use crate::{buffers::msg_head, db::{self, DEFAULT_FETCH_AMOUNT, MAX_FETCH_AMOUNT}, helpers::Res};

pub const URL: &str = "127.0.0.1:7878";

lazy_static::lazy_static! {
    pub static ref PATH: &'static Path = Path::new("/db");
}

pub async fn run_server() {
    let listener = TcpListener::bind(URL).await.unwrap();
    println!("WebSocket server listening on ws://{}", URL);

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(stream));
    }
}

async fn handle_connection(stream: TcpStream) {
    let mut ws_stream = match accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            eprintln!("WebSocket handshake failed: {}", e);
            return;
        }
    };

    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(tungstenite::Message::Binary(data)) => {
                handle_bytes(data).await;
            }
            Ok(tungstenite::Message::Text(text)) => {
                println!("Received text message: {}", text);
            }
            Ok(tungstenite::Message::Close(_)) => {
                println!("Connection closed");
                break;
            }
            Err(e) => {
                eprintln!("Error receiving message: {}", e);
                break;
            }
            _ => {}
        }
    }
}

fn get_chat_file(chat_id: &[u8], data_dir: &Path) -> std::path::PathBuf {
    // encode hash into b64 and append to data_dir
    use base64::{CharacterSet::UrlSafe, Config};
    data_dir.join(base64::encode_config(chat_id, Config::new(UrlSafe, false)))
}

async fn handle_bytes(buffer: Vec<u8>) -> Result<(), &'static str> {
    if buffer.len() < 3 {
        return Err("Buffer too small to match header");
    }

    match &buffer[0..3] {
        b"fch" => {
            // fill fetch buffer
            let mut chat_id_buf = &buffer[3..35];
            let mut pad_buf = &buffer[35..38];
            if pad_buf != b"end" {
                return Err("Incorrect end padding (fch)");
            }

            // get arguments for the db fetch
            let path = get_chat_file(&chat_id_buf, &PATH);

            // fetch from db & send to client
            let (count, msg_id, messages) = db::fetch(&path, DEFAULT_FETCH_AMOUNT)?;
            send_messages(&chat_id_buf, msg_id, true, count, messages)?;
        }
        /*b"qry" => {
            // fill chat_id and arg buffer
            // TODO: read in one go, then split with buffers?
            read_exact(
                &mut stream,
                &mut chat_id_buf,
                "Failed to read query chat id",
            )?;
            read_exact(&mut stream, &mut qry_arg_buf, "Failed to read query args")?;
            read_exact(&mut stream, &mut pad_buf, "Failed to read end pad (qry)")?;
            if pad_buf != pad::END_PADDING {
                return Err("Incorrect end padding (qry)");
            }

            // get arguments for the db fetch
            let (msg_id, count, forward) = query_bytes_to_args(&qry_arg_buf);
            let path = get_chat_file(&chat_id_buf, &globals.data_dir);

            // return query
            let (count, msg_id, messages) = db::query(&path, msg_id, count, forward)?;
            send_messages(&mut stream, &chat_id_buf, msg_id, forward, count, messages)?;
        }*/
        _ => return Err("Recieved invalid SMRT header"),
    };
    Ok(())
}

fn send_messages(
    chat_id: &[u8; 32], // chat_id is 32 bytes long
    msg_id: u32, // id of first message in msgs
    forward: bool,
    count: u8,
    msgs: Vec<u8>,
) -> Res {
    // converts MessageSt to MessageOut and sends each into stream
    // msg::storage_to_packet
    // TcpStream::write
    if count > 127 {
        return Err("Tried to send too many messages");
    }

    // Use max size buffer - size not known, but stack is big anyway
    let mut buffer = [0; msg_head::SIZE + msg_out::SIZE * MAX_FETCH_AMOUNT as usize];
    let (
        // this is horrible but idk how I could format it better...
        buf_pad,
        buf_chat_id,
        buf_msg_id,
        buf_count,
    ) = msg_head::split_mut((&mut buffer[..msg_head::SIZE]).try_into().unwrap());
    // ^ can't fail, perfect size

    // construct header for messages
    buf_pad.copy_from_slice(&msg_head::PAD);
    buf_chat_id.copy_from_slice(&chat_id[..1]);
    buf_msg_id.copy_from_slice(&msg_id.to_be_bytes()[1..]);
    buf_count[0] = (u8::from(forward) << 7) | count;

    // fill buffer with messages
    buffer[msg_head::SIZE..][..msgs.len()].copy_from_slice(&msgs);

    // send
    return Ok(&buffer[..msg_head::SIZE + count as usize * msg_out::SIZE]),
}
