use tokio::time;
use std::error::Error;
use std::collections::BinaryHeap;
use async_trait::async_trait;

pub struct Schedule<T> {
    pub job: T,
    pub deadline: time::Instant,
}

impl<T> Schedule<T> {
    pub fn new(job: T, deadline: time::Instant) -> Self {
        Schedule { job, deadline }
    }

    pub fn now(job: T) -> Self {
        Schedule {
            job,
            deadline: time::Instant::now(),
        }
    }
}

impl<T> PartialEq for Schedule<T> {
    fn eq(&self, other: &Self) -> bool {
        self.deadline == other.deadline
    }
}

impl<T> Eq for Schedule<T> {}

impl<T> PartialOrd for Schedule<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.deadline
            .partial_cmp(&other.deadline)
            .map(|ordering| ordering.reverse())
    }
}

impl<T> Ord for Schedule<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.deadline.cmp(&other.deadline).reverse()
    }
}

#[async_trait]
pub trait ScheduleQueue<T> {
    fn get_late_schedule(&mut self) -> Option<T>;

    async fn wait_first_schedule(&self) -> Result<(), Box<dyn Error>> ;
}

#[async_trait]
impl<T : Sync + Send> ScheduleQueue<T> for BinaryHeap<Schedule<T>> {
    fn get_late_schedule(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let first_schedule = self.peek().unwrap();

        if first_schedule.deadline > time::Instant::now() {
            return None;
        }

        Some(self.pop().unwrap().job)
    }

    async fn wait_first_schedule(&self) -> Result<(), Box<dyn Error>> {
        if self.is_empty() {
            return Err("no schedule".into());
        }

        let first_schedule = self.peek().unwrap();

        time::sleep_until(first_schedule.deadline).await;

        Ok(())
    }
}