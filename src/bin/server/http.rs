use std::{io::Read, net::TcpStream, sync::Arc};

use crate::smrt;
use crate::ws::WsStream;

use publichat::helpers::*;

fn send_code(code: u16, stream: &mut TcpStream) -> Res {
    full_write(
        stream,
        format!("HTTP/1.1 {}\r\n\r\n", code).as_bytes(),
        "Failed to send HTTP status code",
    )
}

fn send_data(code: u16, data: &[u8], stream: &mut TcpStream) -> Res {
    let header_string = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\n\r\n",
        code,
        data.len(),
    );

    full_write(
        stream,
        &[header_string.as_bytes(), data].concat(),
        "Failed to send file",
    )
}

fn handle_http(stream: &mut TcpStream) -> Res {
    let html = format!("<html><body>Running</body></html>");
    send_data(200, html.as_bytes(), stream)
}

fn handle_ws(req: &str, mut stream: TcpStream, globals: &Arc<Globals>) -> Res {
    // handshake
    let key_in = match req.split("Sec-WebSocket-Key: ").nth(1) {
        Some(val) => &val[..24],
        _ => {
            send_code(400, &mut stream)?;
            return Err("Couldn't find WS key");
        }
    };
    WsStream::handshake(&mut stream, key_in)?;

    // launch SMRT
    let mut stream = WsStream::new(stream);
    smrt::handle(&mut stream, globals)
}

pub fn handle(mut stream: TcpStream, globals: &Arc<Globals>) -> Res {
    // Handles GET requests
    let mut buf = [0; 1024]; // todo: think more about sizes
    stream
        .read(&mut buf)
        .map_err(|_| "Failed to read HTTP packet")?;
    let req = std::str::from_utf8(&buf).map_err(|_| "Recieved non-utf8 HTTP")?;

    if !req.ends_with("\0\0\0\0\0\0\0\0") {
        // Received HTTP packet was (probably) bigger than 1 KiB
        send_code(413, &mut stream)?;
        return Err("Received very large HTTP packet; aborted.");
    }

    let path = match req.split(' ').nth(1) {
        // path is 2nd word of GET
        Some(p) => p,
        None => return Err("Failed to find HTTP path"), // faulty HTTP
    };

    match path {
        "/" | "" => handle_http(&mut stream),
        "/ws" => handle_ws(req, stream, globals),
        _ => Ok(()),
    }
}
