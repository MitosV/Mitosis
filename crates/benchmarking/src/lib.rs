pub mod replayer;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::info;

pub use replayer::{RecordedEvent, RecordedSession};
use mitosis_bot_runtime::{BotAction, BotGroup};
use mitosis_metrics::MetricsCollector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkScenario {
    pub name: String,
    pub duration_secs: u64,
    pub ramp_up_secs: u64,
    pub bot_count: usize,
    pub action_type: String, // "afk", "movement", "jump", "stress"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub scenario_name: String,
    pub bot_count: usize,
    pub duration_secs: u64,
    pub avg_tps: f32,
    pub avg_mspt: f32,
    pub peak_cpu_percent: f32,
    pub peak_memory_mb: u64,
    pub packets_per_sec: f64,
    pub success: bool,
}

pub struct BenchmarkRunner {
    metrics: MetricsCollector,
    running: Arc<AtomicBool>,
}

impl BenchmarkRunner {
    pub fn new(metrics: MetricsCollector) -> Self {
        Self {
            metrics,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Executes a benchmark scenario against an active bot group.
    pub async fn run_scenario(
        &self,
        scenario: BenchmarkScenario,
        group: &BotGroup,
    ) -> BenchmarkResult {
        self.running.store(true, Ordering::SeqCst);
        info!("[Benchmark] Starting scenario '{}' with {} bots for {}s", scenario.name, scenario.bot_count, scenario.duration_secs);

        let start_time = Instant::now();
        let mut peak_cpu = 0.0f32;
        let mut peak_mem = 0u64;
        let mut tps_samples = Vec::new();
        let mut mspt_samples = Vec::new();

        let initial_packets = self.metrics.snapshot(group.bot_count(), group.bot_count()).packets_sent_total;

        let end_time = start_time + Duration::from_secs(scenario.duration_secs);

        while Instant::now() < end_time && self.running.load(Ordering::SeqCst) {
            let snap = self.metrics.snapshot(group.bot_count(), group.bot_count());
            if snap.cpu_usage_percent > peak_cpu {
                peak_cpu = snap.cpu_usage_percent;
            }
            if snap.used_memory_mb > peak_mem {
                peak_mem = snap.used_memory_mb;
            }
            tps_samples.push(snap.server_tps);
            mspt_samples.push(snap.server_mspt);

            // Dispatch scenario actions
            match scenario.action_type.as_str() {
                "movement" => {
                    group.send_action_all(BotAction::Move { direction: "forward".into() }).await;
                }
                "jump" => {
                    group.send_action_all(BotAction::Jump).await;
                }
                "stress" => {
                    group.send_action_all(BotAction::Jump).await;
                    group.send_action_all(BotAction::Move { direction: "forward".into() }).await;
                    group.send_action_all(BotAction::Attack { continuous: true }).await;
                }
                _ => {} // AFK
            }

            sleep(Duration::from_millis(500)).await;
        }

        let elapsed_secs = start_time.elapsed().as_secs_f64().max(0.1);
        let final_packets = self.metrics.snapshot(group.bot_count(), group.bot_count()).packets_sent_total;
        let packets_per_sec = (final_packets.saturating_sub(initial_packets) as f64) / elapsed_secs;

        let avg_tps = if !tps_samples.is_empty() {
            tps_samples.iter().sum::<f32>() / tps_samples.len() as f32
        } else {
            20.0
        };

        let avg_mspt = if !mspt_samples.is_empty() {
            mspt_samples.iter().sum::<f32>() / mspt_samples.len() as f32
        } else {
            5.0
        };

        self.running.store(false, Ordering::SeqCst);
        info!("[Benchmark] Scenario '{}' finished. Peak CPU: {:.1}%, Peak RAM: {} MB", scenario.name, peak_cpu, peak_mem);

        BenchmarkResult {
            scenario_name: scenario.name,
            bot_count: scenario.bot_count,
            duration_secs: scenario.duration_secs,
            avg_tps,
            avg_mspt,
            peak_cpu_percent: peak_cpu,
            peak_memory_mb: peak_mem,
            packets_per_sec,
            success: true,
        }
    }
}
