use std::io;

use tokio::net::TcpStream;

pub trait StreamReader {
    fn try_read_until_block(&self, buf: &mut [u8]) -> io::Result<usize>;
}

impl StreamReader for TcpStream {
    fn try_read_until_block(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut result = 0;
    
        loop {
            match self.try_read(buf) {
                Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
                Ok(n) => {
                    result += n;

                    if buf.len() < result {
                        return Err(io::Error::new(io::ErrorKind::Other, "Too large payload"))
                    }
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    break;
                }
                Err(e) => return Err(e),
            };
        }
    
        Ok(result)
    }
}