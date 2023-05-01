use std::net::SocketAddr;

use tokio::net::TcpStream;

pub enum From {
    Client,
    Node,
}

pub enum Job {
    Accept(TcpStream, From),
    Read(SocketAddr, From),
    Drop(SocketAddr, From),
}