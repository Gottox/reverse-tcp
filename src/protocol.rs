use crate::challenge_response::{Challenge, Response};
use crate::io_utils::transfer;
use futures::try_join;
use std::error::Error;
use std::fmt;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_timer::Timeout;

#[derive(Debug)]
pub struct ProtocolError;

impl Error for ProtocolError {}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Protocol Error")
    }
}

pub struct Protocol<T> {
    stream: TcpStream,
    state: T,
}
pub struct Unauthenticated;
pub struct Authenticated;
pub struct Connected;

pub fn from_stream(stream: TcpStream) -> Protocol<Unauthenticated> {
    Protocol {
        stream: stream,
        state: Unauthenticated {},
    }
}

impl Protocol<Unauthenticated> {
    pub async fn authenticate(
        mut self: Self,
        psk: &[u8],
    ) -> Result<Protocol<Authenticated>, Box<dyn Error + Send + Sync>> {
        let (mut rev_read, mut rev_write) = self.stream.split();

        let my_challenge = Challenge::create(&psk)?;

        let (remote_challenge, _) = try_join!(
            Challenge::from_stream(&psk, &mut rev_read),
            my_challenge.send(&mut rev_write)
        )?;

        let my_response = remote_challenge.to_response();

        let (remote_response, _) = try_join!(
            Response::from_stream(&mut rev_read),
            my_response.respond(&mut rev_write)
        )?;

        match my_challenge.to_response().check(&remote_response) {
            Ok(_) => Ok(Protocol {
                stream: self.stream,
                state: Authenticated,
            }),
            Err(e) => {
                let _ =
                    Timeout::new(self.stream.read_to_end(&mut vec![]), Duration::new(5, 0)).await?;
                Err(e)
            }
        }
    }
}

impl Protocol<Authenticated> {
    pub async fn wait_for_connection(
        mut self: Self,
    ) -> Result<Protocol<Connected>, Box<dyn Error + Send + Sync>> {
        let mut magic_bytes = [0; 7];
        self.stream.read_exact(&mut magic_bytes).await?;

        if &magic_bytes != b"connect" {
            return Err(Box::new(ProtocolError {}));
        }

        Ok(Protocol {
            stream: self.stream,
            state: Connected,
        })
    }

    pub async fn connection_available(
        mut self: Self,
    ) -> Result<Protocol<Connected>, Box<dyn Error + Send + Sync>> {
        self.stream.write(b"connect").await?;

        Ok(Protocol {
            stream: self.stream,
            state: Connected,
        })
    }
}

impl Protocol<Connected> {
    pub async fn proxy_for(
        self: Self,
        other_stream: TcpStream,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        transfer(self.stream, other_stream).await
    }
}
