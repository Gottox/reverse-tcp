use futures::future::select;
use std::error::Error;
use std::net::Shutdown;
use tokio::io;
use tokio::net::TcpStream;

pub async fn transfer(
    mut s1: TcpStream,
    mut s2: TcpStream,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (mut r1, mut w1) = s1.split();
    let (mut r2, mut w2) = s2.split();

    let from_1_to_2 = io::copy(&mut r1, &mut w2);
    let from_2_to_1 = io::copy(&mut r2, &mut w1);

    let (result, _) = select(from_1_to_2, from_2_to_1).await.into_inner();
    s1.shutdown(Shutdown::Both)?;
    s2.shutdown(Shutdown::Both)?;
    result?;

    Ok(())
}
