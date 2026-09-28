#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GovernorConfig {
    pub active_input_ms: u64,
    pub cpu_pause_percent: u8,
    pub memory_pause_percent: u8,
    pub cpu_throttle_percent: u8,
    pub idle_sample_ms: u64,
    pub busy_sample_ms: u64,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            active_input_ms: 1_500,
            cpu_pause_percent: 55,
            memory_pause_percent: 88,
            cpu_throttle_percent: 30,
            idle_sample_ms: 1_000,
            busy_sample_ms: 250,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AppConfig {
    pub governor: GovernorConfig,
    pub idle_ram_soft_limit_mb: u32,
    pub idle_ram_hard_limit_mb: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            governor: GovernorConfig::default(),
            idle_ram_soft_limit_mb: 40,
            idle_ram_hard_limit_mb: 64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_keep_hard_limit_above_soft_limit() {
        let cfg = AppConfig::default();
        assert!(cfg.idle_ram_hard_limit_mb >= cfg.idle_ram_soft_limit_mb);
    }
}
