use crate::protocol;
use futures::stream::repeat;
use futures::stream::StreamExt;
use std::error::Error;
use tokio::net::TcpStream;

pub struct ClientConfig {
    pub psk: Vec<u8>,
    pub reverse_server: String,
    pub target: String,
}

pub async fn handle_connection(
    proxy: Result<
        protocol::Protocol<TcpStream, protocol::Authenticated>,
        Box<dyn Error + Send + Sync>,
    >,
    config: &ClientConfig,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if proxy.is_err() {
        return Err(proxy.err().unwrap());
    }
    let proxy_connection = proxy.unwrap().wait_for_connection().await?;
    let target = TcpStream::connect(&config.target).await?;
    tokio::spawn(proxy_connection.proxy_for(target));

    Ok(())
}
pub async fn client(config: ClientConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut proxy_stream = repeat(())
        .then(|_| TcpStream::connect(&config.reverse_server))
        .filter_map(|x| async { x.ok() })
        .then(|stream| protocol::from_stream(stream).authenticate(&config.psk))
        .boxed();

    while let Some(proxy) = proxy_stream.next().await {
        if let Some(err) = handle_connection(proxy, &config).await.err() {
            println!("Warning: {:?}", err.to_string())
        }
    }

    Ok(())
}
