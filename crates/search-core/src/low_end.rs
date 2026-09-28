use crate::{Decision, ResourceGovernor, ResourceSample, WorkClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoClass {
    Hdd,
    Ssd,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryCost {
    Cheap,
    Moderate,
    Expensive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LowEndPolicy {
    pub io_class: IoClass,
    pub result_limit: usize,
    pub background_batch_records: usize,
    pub content_batch_bytes: usize,
    pub fuzzy_candidate_limit: usize,
}

impl LowEndPolicy {
    pub const fn for_storage(io_class: IoClass) -> Self {
        match io_class {
            IoClass::Hdd => Self {
                io_class,
                result_limit: 64,
                background_batch_records: 1024,
                content_batch_bytes: 256 * 1024,
                fuzzy_candidate_limit: 256,
            },
            IoClass::Ssd => Self {
                io_class,
                result_limit: 128,
                background_batch_records: 8192,
                content_batch_bytes: 1024 * 1024,
                fuzzy_candidate_limit: 1024,
            },
            IoClass::Unknown => Self {
                io_class,
                result_limit: 64,
                background_batch_records: 2048,
                content_batch_bytes: 512 * 1024,
                fuzzy_candidate_limit: 512,
            },
        }
    }

    pub fn permit(
        &self,
        governor: &ResourceGovernor,
        cost: QueryCost,
        sample: ResourceSample,
    ) -> Decision {
        match cost {
            QueryCost::Cheap => Decision::Run,
            QueryCost::Moderate => governor.decide(WorkClass::MetadataUpdate, sample),
            QueryCost::Expensive => {
                let base = governor.decide(WorkClass::ContentParse, sample);
                if self.io_class == IoClass::Hdd && sample.user_idle_ms < 5_000 {
                    Decision::Pause
                } else {
                    base
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GovernorConfig;

    #[test]
    fn hdd_expensive_work_pauses_while_user_is_recently_active() {
        let policy = LowEndPolicy::for_storage(IoClass::Hdd);
        let governor = ResourceGovernor::new(GovernorConfig::default());
        let sample = ResourceSample {
            cpu_busy_percent: 2.0,
            memory_load_percent: 20,
            user_idle_ms: 3_000,
            on_battery: false,
        };
        assert_eq!(
            policy.permit(&governor, QueryCost::Expensive, sample),
            Decision::Pause
        );
    }

    #[test]
    fn cheap_query_always_runs() {
        let policy = LowEndPolicy::for_storage(IoClass::Hdd);
        let governor = ResourceGovernor::new(GovernorConfig::default());
        let sample = ResourceSample {
            cpu_busy_percent: 95.0,
            memory_load_percent: 95,
            user_idle_ms: 0,
            on_battery: true,
        };
        assert_eq!(
            policy.permit(&governor, QueryCost::Cheap, sample),
            Decision::Run
        );
    }
}
