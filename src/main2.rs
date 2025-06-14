use std::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    path::Path,
    sync::Arc,
    thread::{self, Builder},
};

mod db;
mod http;
mod smrt;
mod ws;

use publichat::helpers::*;

const IP_PORT_DEFAULT: &str = "localhost:7878";

fn handle_incoming(mut stream: TcpStream, globals: &Arc<Globals>) -> Res {
    let mut pad_buf = [0; 4];

    let mut http_handled: u8 = 0;
    while {
        stream
            .peek(&mut pad_buf)
            .map_err(|_| "Failed to read protocol header (HTTP timeout?)")?;
        &pad_buf == b"GET "
    } {
        http::handle(stream.try_clone().map_err(|_| "Failed to clone")?, globals)?;

        http_handled += 1; // TODO: better system for dropping connections
        if http_handled >= 3 {
            stream
                .shutdown(std::net::Shutdown::Both)
                .map_err(|_| "HTTP shutdown failed")?;
            return Ok(());
        }
        if http_handled == 1 {
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(1)))
                .map_err(|_| "Failed to set short timeout")?;
        }
    }

    // HTTP finished. Read either SMRT or fail.
    if &pad_buf == b"SMRT" {
        read_exact(&mut stream, &mut pad_buf, "Failed to remove SMRT buffer")?;
        smrt::handle(stream, globals)
    } else {
        Err("Failed to match protocol header")
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let globals = {
        // Get chat directory path (last argument)
        let data_dir = {
            if let Some(path) = args.last() {
                let path = Path::new(path);
                if !path.is_dir() {
                    println!("Not a directory: {:?}", path);
                    std::process::exit(1);
                }
                path.to_path_buf() // put on heap
            } else {
                println!("No path given");
                std::process::exit(1);
            }
        };
        println!("Using directory {:?}", data_dir.canonicalize().unwrap());

        Arc::new(Globals { data_dir })
    };

    let listener = {
        let addr = args
            .iter()
            .rev()
            .nth(1)
            .ok_or("Address not given in args")
            .and_then(|arg| {
                arg.to_socket_addrs().map_err(|e| {
                    println!("\t{e}");
                    "Invalid addr, see above"
                })
            })
            .and_then(|mut addrs| addrs.next().ok_or("Empty addr iterator?"))
            .unwrap_or_else(|e| {
                println!("{e}; using default socket address...");
                IP_PORT_DEFAULT
                    .to_socket_addrs()
                    .ok()
                    .and_then(|mut addrs| addrs.next())
                    .unwrap_or_else(|| {
                        println!("Failed to create socket address from default!");
                        println!("Why is {IP_PORT_DEFAULT} an invalid socket addr?");
                        std::process::exit(1);
                    })
            });

        TcpListener::bind(addr).unwrap_or_else(|e| {
            println!("Failed to bind TCP address {addr}:\n\t{e}");
            std::process::exit(1);
        })
    };
    println!("Running on {}", listener.local_addr().unwrap());

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let globals = globals.clone();

            let name = match stream.peer_addr() {
                Ok(addr) => addr.to_string(),
                Err(_) => "unknown".to_string(),
            };

            let builder = Builder::new().name(name);
            let handle = builder.spawn(move || {
                println!("Handling {}", thread::current().name().unwrap());
                if let Err(e) = handle_incoming(stream, &globals) {
                    println!(
                        "Finished {} with:\n\t{e}",
                        thread::current().name().unwrap(),
                    );
                } else {
                    println!(
                        "Finished {} (no message)",
                        thread::current().name().unwrap(),
                    );
                }
            });

            if let Err(e) = handle {
                println!("Failed to create thread: {e}");
            }
        } else {
            println!("failed to bind stream: {}", stream.err().unwrap());
        }
    }
}
