use std::{collections::HashMap, net::SocketAddr, error::Error};
use async_trait::async_trait;
use futures::future::select_all;
use tokio::net::TcpStream;

#[async_trait]
pub trait StreamSelector {
    async fn wait_for_readable(&mut self) -> Result<SocketAddr, Box<dyn Error>>;
}

#[async_trait]
impl StreamSelector for HashMap<SocketAddr, TcpStream> {
    async fn wait_for_readable(&mut self) -> Result<SocketAddr, Box<dyn Error>> {
        if self.is_empty() {
            return Err("no connections".into());
        }

        match select_all(self.iter_mut().map(|(addr, stream)| {
            Box::pin(async {
                stream.readable().await?;

                Ok::<SocketAddr, Box<dyn Error>>(addr.clone())
            })
        }))
        .await
        {
            (Ok(id), _, _) => Ok(id),
            (Err(e), _, _) => Err(e),
        }
    }
}