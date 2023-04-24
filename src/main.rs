use std::collections::{BinaryHeap, HashMap};
use std::error::Error;
use std::io;
use std::net::SocketAddr;

use futures::future::select_all;
use tcp_server::job::{Job, Schedule};
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut schedule_queue = BinaryHeap::new();

    let mut connections = HashMap::new();

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    loop {
        let job = if let Some(job) = get_late_schedule(&mut schedule_queue) {
            job
        } else {
            tokio::select! {
                Ok((stream, _)) = listener.accept() => {
                    Job::Accept(stream)
                }
                Ok(_) = wait_first_schedule(&schedule_queue) => {
                    schedule_queue.pop().unwrap().job
                },
                Ok(addr) = select_from_connections(&mut connections) => {
                    Job::Read(addr)
                },
            }
        };

        match job {
            Job::Accept(stream) => {
                let addr = stream.peer_addr().unwrap();

                println!("{addr:?} accepted");

                connections.insert(addr, stream);
            }
            Job::Read(addr) => {
                if let Some(stream) = connections.get_mut(&addr) {
                    let mut buf = [0; 1024];

                    let n = match read_from(stream, &mut buf) {
                        Ok(n) => n,
                        Err(e) => {
                            eprintln!("{e}");

                            schedule_queue.push(Schedule::now(Job::Drop(addr)));

                            continue;
                        }
                    };

                    let value = String::from_utf8_lossy(&buf[..n]);

                    let lines: Vec<&str> = value.split("\r").collect();

                    if let Some(line) = lines.first() {
                        let tokens: Vec<&str> = line.split(" ").collect();

                        let method = tokens[0];

                        if method != "GET" {
                            continue;
                        }

                        let url = tokens[1];

                        for (addr, stream) in connections.iter_mut() {
                            println!("{addr}");

                            let _ = stream.write_all(format!("{url}\r").as_bytes()).await;
                        }
                    }
                }
            }
            Job::Drop(addr) => {
                connections.remove(&addr);

                println!("{addr:?} dropped");
            }
        }
    }
}

fn read_from(stream: &mut TcpStream, buf: &mut [u8]) -> io::Result<usize> {
    let mut result = 0;

    loop {
        match stream.try_read(buf) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
            Ok(n) => {
                result += n;
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                break;
            }
            Err(e) => return Err(io::Error::from(e)),
        };
    }

    Ok(result)
}

fn get_late_schedule(schedule_queue: &mut BinaryHeap<Schedule<Job>>) -> Option<Job> {
    if schedule_queue.is_empty() {
        return None;
    }

    let first_schedule = schedule_queue.peek().unwrap();

    if first_schedule.deadline > time::Instant::now() {
        return None;
    }

    Some(schedule_queue.pop().unwrap().job)
}

async fn wait_first_schedule(
    schedule_queue: &BinaryHeap<Schedule<Job>>,
) -> Result<(), Box<dyn Error>> {
    if schedule_queue.is_empty() {
        return Err("no schedule".into());
    }

    let first_schedule = schedule_queue.peek().unwrap();

    time::sleep_until(first_schedule.deadline).await;

    Ok(())
}

async fn select_from_connections(
    connections: &mut HashMap<SocketAddr, TcpStream>,
) -> Result<SocketAddr, Box<dyn Error>> {
    if connections.is_empty() {
        return Err("no connections".into());
    }

    match select_all(connections.iter_mut().map(|(id, stream)| {
        Box::pin(async {
            stream.readable().await?;

            Ok::<SocketAddr, Box<dyn Error>>(stream.peer_addr().unwrap())
        })
    }))
    .await
    {
        (Ok(id), _, _) => Ok(id),
        (Err(e), _, _) => Err(e),
    }
}
