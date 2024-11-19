pub mod decoder;
pub mod encoder;
pub mod server;
pub mod store;

use anyhow::Result;
use crate::server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    let server = Server::new("127.0.0.1:6379").await?;
    server.run().await
}
