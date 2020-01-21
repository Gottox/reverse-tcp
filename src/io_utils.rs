use std::error::Error;
use futures::future::{select, Either};
use tokio::io;
use tokio::io::{AsyncRead, AsyncWrite};

pub async fn transfer<T1, T2>(
    mut s1: T1,
    mut s2: T2,
) -> Result<(), Box<dyn Error + Send + Sync>>
where T1: AsyncRead + AsyncWrite + Unpin,
      T2: AsyncRead + AsyncWrite + Unpin,
{
    let (mut r1, mut w1) = io::split(&mut s1);
    let (mut r2, mut w2) = io::split(&mut s2);

    let from_1_to_2 = io::copy(&mut r1, &mut w2);
    let from_2_to_1 = io::copy(&mut r2, &mut w1);

    match select(from_1_to_2, from_2_to_1).await {
        Either::Left((x, _)) => x,
        Either::Right((x, _)) => x,
    }?;

    Ok(())
}
