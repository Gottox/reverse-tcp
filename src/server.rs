use crate::protocol;
use futures::stream::StreamExt;
use std::error::Error;
use tokio::net::TcpListener;

pub struct ServerConfig {
    pub psk: Vec<u8>,
    pub reverse_port: String,
    pub target: String,
}

pub async fn server(config: ServerConfig) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut rev_listener = TcpListener::bind(&config.reverse_port).await?;
    let mut target_listener = TcpListener::bind(&config.target).await?;

    let mut target_stream = target_listener
        .incoming()
        .filter_map(|x| async { x.ok() })
        .boxed();
    let mut proxy_stream = rev_listener
        .incoming()
        .filter_map(|x| async { x.ok() })
        .then(|stream| protocol::from_stream(stream).authenticate(&config.psk))
        .filter_map(|x| async { x.ok() })
        .boxed();

    while let Some(target) = target_stream.next().await {
        tokio::spawn(
            proxy_stream
                .next()
                .await
                .unwrap()
                .connection_available()
                .await
                .unwrap()
                .proxy_for(target),
        );
    }

    Ok(())
}
