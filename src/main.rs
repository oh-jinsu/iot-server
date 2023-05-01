use std::collections::{BinaryHeap, HashMap};
use std::error::Error;
use tcp_server::job::{Job, From};
use tcp_server::schedule::{Schedule, ScheduleQueue};
use tcp_server::selector::StreamSelector;
use tcp_server::stream::StreamReader;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut schedule_queue = BinaryHeap::new();

    let mut clients = HashMap::new();

    let mut nodes = HashMap::new();

    let listener_for_clients = TcpListener::bind("0.0.0.0:3000").await?;

    let listener_for_nodes = TcpListener::bind("0.0.0.0:3001").await?;

    loop {
        let job = match schedule_queue.get_late_schedule() {
            Some(job) => job,
            None => tokio::select! {
                Ok(_) = schedule_queue.wait_first_schedule() => {
                    schedule_queue.pop().unwrap().job
                },
                Ok((stream, _)) = listener_for_clients.accept() => {
                    Job::Accept(stream, From::Client)
                }
                Ok(addr) = clients.wait_for_readable() => {
                    Job::Read(addr, From::Client)
                },
                Ok((stream, _)) = listener_for_nodes.accept() => {
                    Job::Accept(stream, From::Node)
                }
                Ok(addr) = nodes.wait_for_readable() => {
                    Job::Read(addr, From::Node)
                },
            }
        };

        match job {
            Job::Accept(stream, from) => {
                let Ok(addr) = stream.peer_addr() else {
                    continue;
                };
                
                match from {
                    From::Client => clients.insert(addr, stream),
                    From::Node => nodes.insert(addr, stream),
                };

                println!("{addr:?} accepted");
            }
            Job::Drop(addr, from) => {
                match from {
                    From::Client => clients.remove(&addr),
                    From::Node => nodes.remove(&addr),
                };

                println!("{addr:?} dropped");
            }
            Job::Read(addr, from) => {
                let Some(stream) = (match from {
                    From::Client => clients.get_mut(&addr),
                    From::Node => nodes.get_mut(&addr),
                }) else {
                    continue;
                };

                let mut buf = [0; 1024];

                let n = match stream.try_read_until_block(&mut buf) {
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("{e}");

                        schedule_queue.push(Schedule::now(Job::Drop(addr, from)));

                        continue;
                    }
                };

                let value = String::from_utf8_lossy(&buf[..n]);

                let lines: Vec<&str> = value.split("\r").collect();

                let Some(line) = lines.first() else {
                    continue;
                };

                let tokens: Vec<&str> = line.split(" ").collect();

                let method = tokens[0];

                if method != "GET" {
                    continue;
                }

                let url = tokens[1];

                for (addr, stream) in nodes.iter_mut() {
                    if let Err(e) = stream.write_all(format!("{url}\r").as_bytes()).await {
                        eprintln!("{e}");

                        schedule_queue.push(Schedule::now(Job::Drop(*addr, From::Node)));

                        continue;
                    }
                }
            }
        }
    }
}
