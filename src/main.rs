use redis_starter_rust::config::Args;
use redis_starter_rust::server::Server;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let _args = Args::load()?;

    let server = Server::new("127.0.0.1:6379").await?;
    server.run().await
}
