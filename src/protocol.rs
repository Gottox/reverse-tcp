use crate::challenge_response::{Challenge, Response};
use crate::io_utils::transfer;
use futures::try_join;
use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::time::Duration;
use tokio::io;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_timer::Timeout;

#[derive(Debug)]
pub struct ProtocolError;

impl Error for ProtocolError {}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Protocol Error")
    }
}

pub struct Protocol<S, M>
where
    S: AsyncRead + AsyncWrite,
{
    stream: S,
    state: PhantomData<M>,
}
pub struct Unauthenticated;
pub struct Authenticated;
pub struct Connected;

pub fn from_stream<S>(stream: S) -> Protocol<S, Unauthenticated>
where
    S: AsyncRead + AsyncWrite,
{
    Protocol {
        stream,
        state: PhantomData::<Unauthenticated>,
    }
}

impl<S> Protocol<S, Unauthenticated>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn authenticate(
        mut self: Self,
        psk: &[u8],
    ) -> Result<Protocol<S, Authenticated>, Box<dyn Error + Send + Sync>> {
        let (mut rev_read, mut rev_write) = io::split(&mut self.stream);

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
                state: PhantomData::<Authenticated>,
            }),
            Err(e) => {
                let _ =
                    Timeout::new(self.stream.read_to_end(&mut vec![]), Duration::new(5, 0)).await?;
                Err(e)
            }
        }
    }
}

impl<S> Protocol<S, Authenticated>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn wait_for_connection(
        mut self: Self,
    ) -> Result<Protocol<S, Connected>, Box<dyn Error + Send + Sync>> {
        let mut magic_bytes = [0; 7];
        self.stream.read_exact(&mut magic_bytes).await?;

        if &magic_bytes != b"connect" {
            return Err(Box::new(ProtocolError {}));
        }

        Ok(Protocol {
            stream: self.stream,
            state: PhantomData::<Connected>,
        })
    }

    pub async fn connection_available(
        mut self: Self,
    ) -> Result<Protocol<S, Connected>, Box<dyn Error + Send + Sync>> {
        self.stream.write(b"connect").await?;

        Ok(Protocol {
            stream: self.stream,
            state: PhantomData::<Connected>,
        })
    }
}

impl<S> Protocol<S, Connected>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn proxy_for<S2>(
        self: Self,
        other_stream: S2,
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        S2: AsyncRead + AsyncWrite + Unpin,
    {
        transfer(self.stream, other_stream).await
    }
}
