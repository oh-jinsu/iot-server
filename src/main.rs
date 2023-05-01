use std::collections::{BinaryHeap, HashMap};
use std::error::Error;
use tcp_server::job::Job;
use tcp_server::schedule::{Schedule, ScheduleQueue};
use tcp_server::selector::StreamSelector;
use tcp_server::stream::StreamReader;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut schedule_queue = BinaryHeap::new();

    let mut connections = HashMap::new();

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    loop {
        let job = match schedule_queue.get_late_schedule() {
            Some(job) => job,
            None => tokio::select! {
                Ok((stream, _)) = listener.accept() => {
                    Job::Accept(stream)
                }
                Ok(_) = schedule_queue.wait_first_schedule() => {
                    schedule_queue.pop().unwrap().job
                },
                Ok(addr) = connections.wait_for_readable() => {
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
                let Some(stream) = connections.get_mut(&addr) else {
                    continue;
                };
                
                let mut buf = [0; 1024];

                let n = match stream.try_read_until_block(&mut buf) {
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("{e}");

                        schedule_queue.push(Schedule::now(Job::Drop(addr)));

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

                for (_, stream) in connections.iter_mut() {
                    let _ = stream.write_all(format!("{url}\r").as_bytes()).await;
                }
            }
            Job::Drop(addr) => {
                connections.remove(&addr);

                println!("{addr:?} dropped");
            }
        }
    }
}
