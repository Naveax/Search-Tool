use crate::GovernorConfig;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceSample {
    pub cpu_busy_percent: f32,
    pub memory_load_percent: u8,
    pub user_idle_ms: u64,
    pub on_battery: bool,
}

impl Default for ResourceSample {
    fn default() -> Self {
        Self {
            cpu_busy_percent: 0.0,
            memory_load_percent: 0,
            user_idle_ms: u64::MAX,
            on_battery: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkClass {
    ForegroundQuery,
    MetadataUpdate,
    BackgroundIndex,
    ContentParse,
    AiInference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Run,
    Throttle,
    Pause,
}

#[derive(Debug, Clone)]
pub struct ResourceGovernor {
    cfg: GovernorConfig,
}

impl ResourceGovernor {
    pub const fn new(cfg: GovernorConfig) -> Self {
        Self { cfg }
    }

    pub fn decide(&self, class: WorkClass, sample: ResourceSample) -> Decision {
        if class == WorkClass::ForegroundQuery {
            return Decision::Run;
        }

        if sample.memory_load_percent >= self.cfg.memory_pause_percent {
            return Decision::Pause;
        }

        if sample.cpu_busy_percent >= self.cfg.cpu_pause_percent as f32 {
            return Decision::Pause;
        }

        let user_is_active = sample.user_idle_ms < self.cfg.active_input_ms;

        match class {
            WorkClass::MetadataUpdate => {
                if sample.cpu_busy_percent >= self.cfg.cpu_throttle_percent as f32 {
                    Decision::Throttle
                } else {
                    Decision::Run
                }
            }
            WorkClass::BackgroundIndex | WorkClass::ContentParse | WorkClass::AiInference => {
                if user_is_active || sample.on_battery {
                    Decision::Pause
                } else if sample.cpu_busy_percent >= self.cfg.cpu_throttle_percent as f32 {
                    Decision::Throttle
                } else {
                    Decision::Run
                }
            }
            WorkClass::ForegroundQuery => Decision::Run,
        }
    }

    pub const fn idle_sample_ms(&self) -> u64 {
        self.cfg.idle_sample_ms
    }

    pub const fn busy_sample_ms(&self) -> u64 {
        self.cfg.busy_sample_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn governor() -> ResourceGovernor {
        ResourceGovernor::new(GovernorConfig::default())
    }

    #[test]
    fn foreground_query_always_runs() {
        let sample = ResourceSample {
            cpu_busy_percent: 99.0,
            memory_load_percent: 99,
            user_idle_ms: 0,
            on_battery: true,
        };
        assert_eq!(
            governor().decide(WorkClass::ForegroundQuery, sample),
            Decision::Run
        );
    }

    #[test]
    fn background_pauses_while_user_is_active() {
        let sample = ResourceSample {
            cpu_busy_percent: 2.0,
            memory_load_percent: 25,
            user_idle_ms: 200,
            on_battery: false,
        };
        assert_eq!(
            governor().decide(WorkClass::BackgroundIndex, sample),
            Decision::Pause
        );
    }

    #[test]
    fn background_runs_when_machine_is_idle_and_calm() {
        let sample = ResourceSample {
            cpu_busy_percent: 5.0,
            memory_load_percent: 35,
            user_idle_ms: 10_000,
            on_battery: false,
        };
        assert_eq!(
            governor().decide(WorkClass::BackgroundIndex, sample),
            Decision::Run
        );
    }
}
