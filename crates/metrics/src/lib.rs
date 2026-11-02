use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsSnapshot {
    pub cpu_usage_percent: f32,
    pub used_memory_mb: u64,
    pub total_memory_mb: u64,
    pub connected_bots: usize,
    pub total_bots: usize,
    pub packets_sent_total: u64,
    pub packets_received_total: u64,
    pub server_tps: f32,
    pub server_mspt: f32,
}

#[derive(Clone)]
pub struct MetricsCollector {
    sys: Arc<Mutex<System>>,
    packets_sent: Arc<AtomicU64>,
    packets_received: Arc<AtomicU64>,
    server_tps: Arc<Mutex<f32>>,
    server_mspt: Arc<Mutex<f32>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        let sys = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        Self {
            sys: Arc::new(Mutex::new(sys)),
            packets_sent: Arc::new(AtomicU64::new(0)),
            packets_received: Arc::new(AtomicU64::new(0)),
            server_tps: Arc::new(Mutex::new(20.0)),
            server_mspt: Arc::new(Mutex::new(5.0)),
        }
    }

    pub fn record_sent_packet(&self) {
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_received_packet(&self) {
        self.packets_received.fetch_add(1, Ordering::Relaxed);
    }

    pub fn update_server_telemetry(&self, tps: f32, mspt: f32) {
        *self.server_tps.lock().unwrap() = tps;
        *self.server_mspt.lock().unwrap() = mspt;
    }

    pub fn snapshot(&self, connected_bots: usize, total_bots: usize) -> SystemMetricsSnapshot {
        let mut sys = self.sys.lock().unwrap();
        sys.refresh_cpu_all();
        sys.refresh_memory();

        let cpu = sys.global_cpu_usage();
        let used_mem = sys.used_memory() / (1024 * 1024);
        let total_mem = sys.total_memory() / (1024 * 1024);

        SystemMetricsSnapshot {
            cpu_usage_percent: cpu,
            used_memory_mb: used_mem,
            total_memory_mb: total_mem,
            connected_bots,
            total_bots,
            packets_sent_total: self.packets_sent.load(Ordering::Relaxed),
            packets_received_total: self.packets_received.load(Ordering::Relaxed),
            server_tps: *self.server_tps.lock().unwrap(),
            server_mspt: *self.server_mspt.lock().unwrap(),
        }
    }

    /// Generates Prometheus exposition format output.
    pub fn to_prometheus(&self, connected_bots: usize, total_bots: usize) -> String {
        let snap = self.snapshot(connected_bots, total_bots);
        format!(
            "# HELP mitosis_connected_bots Number of actively connected bots\n\
             # TYPE mitosis_connected_bots gauge\n\
             mitosis_connected_bots {}\n\n\
             # HELP mitosis_cpu_usage_percent Process CPU usage percentage\n\
             # TYPE mitosis_cpu_usage_percent gauge\n\
             mitosis_cpu_usage_percent {}\n\n\
             # HELP mitosis_used_memory_mb Process memory usage in megabytes\n\
             # TYPE mitosis_used_memory_mb gauge\n\
             mitosis_used_memory_mb {}\n\n\
             # HELP mitosis_packets_sent_total Total packets transmitted\n\
             # TYPE mitosis_packets_sent_total counter\n\
             mitosis_packets_sent_total {}\n\n\
             # HELP mitosis_packets_received_total Total packets received\n\
             # TYPE mitosis_packets_received_total counter\n\
             mitosis_packets_received_total {}\n\n\
             # HELP mitosis_server_tps Server TPS\n\
             # TYPE mitosis_server_tps gauge\n\
             mitosis_server_tps {}\n",
            snap.connected_bots,
            snap.cpu_usage_percent,
            snap.used_memory_mb,
            snap.packets_sent_total,
            snap.packets_received_total,
            snap.server_tps,
        )
    }
}
