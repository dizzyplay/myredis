pub mod config;
pub mod protocol {
    pub mod decoder;
    pub mod encoder;
}
pub mod server;
pub mod store;

use crate::config::Args;
use crate::server::Server;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let _args = Args::load()?;

    let server = Server::new("127.0.0.1:6379").await?;
    server.run().await
}
