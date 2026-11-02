use std::time::Duration;
use rand::Rng;

#[derive(Debug, Clone)]
pub struct BackoffConfig {
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
    pub jitter_factor: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(30),
            multiplier: 1.5,
            jitter_factor: 0.2,
        }
    }
}

pub struct ExponentialBackoff {
    config: BackoffConfig,
    current_delay: Duration,
    attempts: u32,
}

impl ExponentialBackoff {
    pub fn new(config: BackoffConfig) -> Self {
        let current_delay = config.initial_delay;
        Self {
            config,
            current_delay,
            attempts: 0,
        }
    }

    pub fn next_delay(&mut self) -> Duration {
        self.attempts += 1;
        let base_millis = self.current_delay.as_millis() as f64;
        let next_base = (base_millis * self.config.multiplier).min(self.config.max_delay.as_millis() as f64);
        self.current_delay = Duration::from_millis(next_base as u64);

        // Apply random jitter: +/- jitter_factor
        let mut rng = rand::thread_rng();
        let jitter = rng.gen_range(-self.config.jitter_factor..=self.config.jitter_factor);
        let jittered_millis = (base_millis * (1.0 + jitter)).max(100.0);
        Duration::from_millis(jittered_millis as u64)
    }

    pub fn reset(&mut self) {
        self.attempts = 0;
        self.current_delay = self.config.initial_delay;
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_progression() {
        let mut backoff = ExponentialBackoff::new(BackoffConfig::default());
        let d1 = backoff.next_delay();
        let d2 = backoff.next_delay();
        assert!(d1.as_millis() > 0);
        assert!(d2.as_millis() >= d1.as_millis() / 2);
        backoff.reset();
        assert_eq!(backoff.attempts(), 0);
    }
}
