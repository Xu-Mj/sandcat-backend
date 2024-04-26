use rand::Rng;
use std::fmt::{Debug, Display, Formatter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub fn get_strategy(lb_type: LoadBalanceStrategyType) -> Arc<dyn LoadBalanceStrategy> {
    match lb_type {
        LoadBalanceStrategyType::RoundRobin => Arc::new(RoundRobin::new()),
        LoadBalanceStrategyType::Random => Arc::new(RandomStrategy),
    }
}

pub trait LoadBalanceStrategy: Debug + Send + Sync {
    fn index(&self, service_count: usize) -> usize;
    fn reset(&mut self) {}
}

pub enum LoadBalanceStrategyType {
    RoundRobin,
    Random,
}

impl Display for LoadBalanceStrategyType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadBalanceStrategyType::RoundRobin => write!(f, "RoundRobin"),
            LoadBalanceStrategyType::Random => write!(f, "Random"),
        }
    }
}

impl From<String> for LoadBalanceStrategyType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "RoundRobin" => Self::RoundRobin,
            "Random" => Self::Random,
            _ => Self::RoundRobin,
        }
    }
}

#[derive(Debug)]
pub struct RoundRobin {
    /// counter, record the number of the request
    counter: AtomicUsize,
}

impl LoadBalanceStrategy for RoundRobin {
    fn index(&self, service_count: usize) -> usize {
        let index = self.counter.fetch_add(1, Ordering::Relaxed);
        index % service_count
    }

    fn reset(&mut self) {
        self.counter.store(0, Ordering::Relaxed);
    }
}

impl RoundRobin {
    pub fn new() -> Self {
        Self {
            counter: AtomicUsize::new(0),
        }
    }
}

#[derive(Debug)]
pub struct RandomStrategy;

impl LoadBalanceStrategy for RandomStrategy {
    fn index(&self, service_count: usize) -> usize {
        rand::thread_rng().gen_range(0..service_count)
    }
}
