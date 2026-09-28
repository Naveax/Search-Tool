use crate::knowledge::{Classification, FileClass, RiskLevel};
use crate::store::normalize_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupAction {
    Keep,
    RecommendOnly,
    Quarantine,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleanupDecision {
    pub action: CleanupAction,
    pub confidence: u8,
}

pub fn cleanup_decision(path: &str, classification: Classification) -> CleanupDecision {
    let p = normalize_name(path).replace('/', "\\");
    if is_protected_path(&p) || classification.risk == RiskLevel::Critical {
        return CleanupDecision {
            action: CleanupAction::Deny,
            confidence: 100,
        };
    }
    match classification.class {
        FileClass::Cache | FileClass::Temporary => CleanupDecision {
            action: CleanupAction::Quarantine,
            confidence: classification.confidence,
        },
        FileClass::BuildArtifact | FileClass::Log => CleanupDecision {
            action: CleanupAction::RecommendOnly,
            confidence: classification.confidence,
        },
        FileClass::System | FileClass::Application | FileClass::UserData | FileClass::Project => {
            CleanupDecision {
                action: CleanupAction::Deny,
                confidence: classification.confidence,
            }
        }
        FileClass::Unknown => CleanupDecision {
            action: CleanupAction::Keep,
            confidence: 20,
        },
    }
}

pub fn is_protected_path(normalized_path: &str) -> bool {
    let p = normalized_path;
    p.contains("\\windows\\system32\\")
        || p.contains("\\windows\\winsxs\\")
        || p.contains("\\windows\\servicing\\")
        || p.ends_with("\\windows")
        || p.contains("\\system volume information\\")
        || p.contains("\\program files\\windows defender\\")
        || p.ends_with("\\ntldr")
        || p.ends_with("\\bootmgr")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knowledge::{classify_path, FileClass};

    #[test]
    fn never_recommends_system32_deletion() {
        let path = r"C:\Windows\System32\kernel32.dll";
        assert_eq!(
            cleanup_decision(path, classify_path(path, 0)).action,
            CleanupAction::Deny
        );
    }

    #[test]
    fn cache_goes_to_quarantine_not_direct_delete() {
        let path = r"C:\Users\x\AppData\Local\npm-cache\x";
        let c = classify_path(path, 0);
        assert_eq!(c.class, FileClass::Cache);
        assert_eq!(cleanup_decision(path, c).action, CleanupAction::Quarantine);
    }
}
