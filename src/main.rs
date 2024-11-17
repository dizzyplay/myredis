pub mod command;
pub mod decode;
pub mod server;
pub mod store;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let server = server::Server::new("127.0.0.1:6379").await?;
    server.run().await?;
    Ok(())
}
