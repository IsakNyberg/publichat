#[tokio::main]
async fn main() {
    server::run_server().await;
}

mod server;
