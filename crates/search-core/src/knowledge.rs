use crate::store::{normalize_name, FLAG_SYSTEM};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileClass {
    System,
    Application,
    UserData,
    Project,
    Cache,
    Temporary,
    BuildArtifact,
    Log,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Classification {
    pub class: FileClass,
    pub risk: RiskLevel,
    pub confidence: u8,
}

pub fn classify_path(path: &str, flags: u16) -> Classification {
    let p = normalize_name(path).replace('/', "\\");
    let components: Vec<&str> = p.split('\\').filter(|part| !part.is_empty()).collect();

    if flags & FLAG_SYSTEM != 0
        || p.contains("\\windows\\system32\\")
        || p.contains("\\windows\\winsxs\\")
        || p.contains("\\windows\\servicing\\")
        || p.contains("\\boot\\")
    {
        return Classification {
            class: FileClass::System,
            risk: RiskLevel::Critical,
            confidence: 99,
        };
    }
    if components.iter().any(|c| matches!(*c, ".git" | ".svn"))
        || components.iter().any(|c| matches!(*c, "src" | "source"))
        || p.ends_with("cargo.toml")
        || p.ends_with("package.json")
        || p.ends_with("pyproject.toml")
    {
        return Classification {
            class: FileClass::Project,
            risk: RiskLevel::High,
            confidence: 92,
        };
    }
    if components
        .iter()
        .any(|c| matches!(*c, "node_modules" | "target" | "__pycache__" | ".gradle"))
        || components
            .iter()
            .any(|c| matches!(*c, "dist" | "build" | "obj" | "bin"))
    {
        return Classification {
            class: FileClass::BuildArtifact,
            risk: RiskLevel::Medium,
            confidence: 88,
        };
    }
    if components.iter().any(|c| {
        matches!(
            *c,
            "cache" | "caches" | "npm-cache" | "code cache" | "gpucache"
        )
    }) || p.contains("\\appdata\\local\\temp\\")
    {
        return Classification {
            class: FileClass::Cache,
            risk: RiskLevel::Low,
            confidence: 92,
        };
    }
    if components.iter().any(|c| matches!(*c, "temp" | "tmp"))
        || p.ends_with(".tmp")
        || p.ends_with(".temp")
    {
        return Classification {
            class: FileClass::Temporary,
            risk: RiskLevel::Low,
            confidence: 86,
        };
    }
    if p.ends_with(".log") || p.ends_with(".trace") {
        return Classification {
            class: FileClass::Log,
            risk: RiskLevel::Low,
            confidence: 82,
        };
    }
    if components.iter().any(|c| {
        matches!(
            *c,
            "documents" | "desktop" | "pictures" | "videos" | "music" | "downloads"
        )
    }) {
        return Classification {
            class: FileClass::UserData,
            risk: RiskLevel::High,
            confidence: 85,
        };
    }
    if p.contains("\\program files\\")
        || p.contains("\\program files (x86)\\")
        || p.contains("\\programdata\\")
    {
        return Classification {
            class: FileClass::Application,
            risk: RiskLevel::High,
            confidence: 88,
        };
    }
    Classification {
        class: FileClass::Unknown,
        risk: RiskLevel::High,
        confidence: 25,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recognizes_system_and_cache() {
        assert_eq!(
            classify_path(r"C:\Windows\System32\kernel32.dll", 0).risk,
            RiskLevel::Critical
        );
        assert_eq!(
            classify_path(r"C:\Users\x\AppData\Local\npm-cache\a", 0).class,
            FileClass::Cache
        );
    }
    #[test]
    fn project_beats_generic_build_name() {
        assert_eq!(
            classify_path(r"C:\work\repo\.git\objects\x", 0).class,
            FileClass::Project
        );
    }
}
