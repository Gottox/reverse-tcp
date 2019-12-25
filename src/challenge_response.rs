use sha2::{Digest, Sha256};
use std::error::Error;
use std::fmt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct ResponseError;

impl Error for ResponseError {}

impl fmt::Display for ResponseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bad Response to Challenge")
    }
}

pub struct Challenge {
    psk: Vec<u8>,
    salt: [u8; 32],
}

impl Challenge {
    pub fn create(psk: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut challenge = [0; 32];
        getrandom::getrandom(&mut challenge)?;
        Ok(Self::from_slice(&psk, &challenge))
    }

    pub async fn from_stream<R: AsyncRead + Unpin>(
        psk: &[u8],
        stream: &mut R,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut salt = [0; 32];
        stream.read_exact(&mut salt).await?;
        Ok(Self::from_slice(psk, &salt))
    }

    fn from_slice(psk: &[u8], salt: &[u8]) -> Self {
        let mut challenge = Self {
            psk: psk.to_vec(),
            salt: [0; 32],
        };
        challenge.salt.copy_from_slice(salt);
        challenge
    }

    pub fn to_response(self: &Self) -> Response {
        let hash = Sha256::new().chain(&self.psk).chain(&self.salt).result();
        Response::from_slice(hash.as_slice())
    }

    pub async fn send<W: AsyncWrite + Unpin>(
        self: &Self,
        stream: &mut W,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        stream.write(&self.salt).await?;
        Ok(())
    }
}

#[derive(Eq, PartialEq)]
pub struct Response {
    hash: [u8; 32],
}

impl Response {
    pub async fn from_stream<R: AsyncRead + Unpin>(
        stream: &mut R,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut hash = [0; 32];
        stream.read_exact(&mut hash).await?;
        Ok(Self::from_slice(&hash))
    }

    fn from_slice(hash: &[u8]) -> Self {
        let mut response = Self { hash: [0; 32] };
        response.hash.copy_from_slice(hash);
        response
    }

    pub async fn respond<W: AsyncWrite + Unpin>(
        self: &Self,
        stream: &mut W,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        stream.write(&self.hash).await?;
        Ok(())
    }

    pub fn check(self: &Self, other: &Self) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self == other {
            Ok(())
        } else {
            Err(Box::new(ResponseError))
        }
    }
}
