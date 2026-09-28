use crate::store::normalize_name;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelationQuery {
    pub canonical: &'static str,
    pub aliases: &'static [&'static str],
}

const NODE: RelationQuery = RelationQuery {
    canonical: "node.js",
    aliases: &[
        "node",
        "node.exe",
        "npm",
        "npx",
        "corepack",
        "package.json",
        "package-lock.json",
        "node_modules",
        ".npmrc",
        "npm-cache",
    ],
};
const RUST: RelationQuery = RelationQuery {
    canonical: "rust",
    aliases: &[
        "rustc",
        "cargo",
        "cargo.toml",
        "cargo.lock",
        ".cargo",
        "target",
    ],
};
const PYTHON: RelationQuery = RelationQuery {
    canonical: "python",
    aliases: &[
        "python",
        "python.exe",
        "python3",
        "pip",
        "pip3",
        "pyproject.toml",
        "requirements.txt",
        ".venv",
        "venv",
        "__pycache__",
    ],
};
const GIT: RelationQuery = RelationQuery {
    canonical: "git",
    aliases: &[
        "git",
        "git.exe",
        ".git",
        ".gitignore",
        ".gitattributes",
        ".gitconfig",
    ],
};
const STEAM: RelationQuery = RelationQuery {
    canonical: "steam",
    aliases: &[
        "steam",
        "steam.exe",
        "steamapps",
        "libraryfolders.vdf",
        "appmanifest_",
    ],
};

pub fn relation_for_query(query: &str) -> Option<&'static RelationQuery> {
    let normalized = normalize_name(query);
    let compact = normalized.replace([' ', '-'], "");
    let words: Vec<&str> = normalized.split_whitespace().collect();
    if compact.contains("nodejs") || words.iter().any(|w| matches!(*w, "node" | "npm" | "npx")) {
        return Some(&NODE);
    }
    if compact.contains("rustlang")
        || words
            .iter()
            .any(|w| matches!(*w, "rust" | "rustc" | "cargo"))
    {
        return Some(&RUST);
    }
    if words
        .iter()
        .any(|w| matches!(*w, "python" | "python3" | "pip" | "pip3"))
    {
        return Some(&PYTHON);
    }
    if words.iter().any(|w| matches!(*w, "git" | "github")) {
        return Some(&GIT);
    }
    if words.contains(&"steam") {
        return Some(&STEAM);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn node_js_resolves_to_ecosystem() {
        let r = relation_for_query("Node JS").unwrap();
        assert!(r.aliases.contains(&"package.json"));
        assert!(r.aliases.contains(&"node_modules"));
    }
}
