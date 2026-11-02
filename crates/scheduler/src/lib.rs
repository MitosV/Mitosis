use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc};
use tokio::time::interval;
use tracing::debug;

pub struct TickScheduler {
    interval_ms: u64,
    current_tick: Arc<AtomicU64>,
    drift_micros: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
    tick_tx: broadcast::Sender<u64>,
}

impl TickScheduler {
    pub fn new(tps: u32) -> Self {
        let interval_ms = (1000 / tps) as u64;
        let (tick_tx, _) = broadcast::channel(128);
        Self {
            interval_ms,
            current_tick: Arc::new(AtomicU64::new(0)),
            drift_micros: Arc::new(AtomicU64::new(0)),
            running: Arc::new(AtomicBool::new(true)),
            tick_tx,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<u64> {
        self.tick_tx.subscribe()
    }

    pub fn current_tick(&self) -> u64 {
        self.current_tick.load(Ordering::Relaxed)
    }

    pub fn drift_micros(&self) -> u64 {
        self.drift_micros.load(Ordering::Relaxed)
    }

    pub async fn start(&self) {
        let mut ticker = interval(Duration::from_millis(self.interval_ms));
        let mut expected_time = Instant::now();

        while self.running.load(Ordering::SeqCst) {
            ticker.tick().await;
            let now = Instant::now();
            let drift = if now > expected_time {
                now.duration_since(expected_time).as_micros() as u64
            } else {
                0
            };
            self.drift_micros.store(drift, Ordering::Relaxed);
            expected_time = now + Duration::from_millis(self.interval_ms);

            let tick = self.current_tick.fetch_add(1, Ordering::SeqCst);
            let _ = self.tick_tx.send(tick);
        }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }
}

pub struct WorkerPool {
    task_tx: mpsc::Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl WorkerPool {
    pub fn new(num_workers: usize) -> Self {
        let (task_tx, _task_rx) = mpsc::channel::<Box<dyn FnOnce() + Send + 'static>>(1024);

        for id in 0..num_workers.max(1) {
            tokio::task::spawn_blocking(move || {
                debug!("[Worker {}] Started", id);
            });
        }

        Self { task_tx }
    }

    pub async fn spawn<F>(&self, task: F) -> Result<(), mpsc::error::SendError<Box<dyn FnOnce() + Send + 'static>>>
    where
        F: FnOnce() + Send + 'static,
    {
        self.task_tx.send(Box::new(task)).await
    }
}
