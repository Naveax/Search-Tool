use crate::attributes::AttributeEntry;
use crate::relationship::relation_for_query;
use crate::store::{normalize_name, FLAG_DIRECTORY, FLAG_HIDDEN, FLAG_SYSTEM};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTypeFilter {
    File,
    Directory,
    Hidden,
    System,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchFilters {
    pub extension: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub path_contains: Option<String>,
    pub modified_after: Option<i64>,
    pub modified_before: Option<i64>,
    pub item_type: Option<ItemTypeFilter>,
    pub application: Option<String>,
}

impl SearchFilters {
    pub const fn is_empty(&self) -> bool {
        self.extension.is_none()
            && self.min_size.is_none()
            && self.max_size.is_none()
            && self.path_contains.is_none()
            && self.modified_after.is_none()
            && self.modified_before.is_none()
            && self.item_type.is_none()
            && self.application.is_none()
    }

    pub const fn needs_attributes(&self) -> bool {
        self.min_size.is_some()
            || self.max_size.is_some()
            || self.modified_after.is_some()
            || self.modified_before.is_some()
    }

    pub const fn needs_path(&self) -> bool {
        self.path_contains.is_some() || self.application.is_some()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParsedSearchQuery {
    pub text: String,
    pub filters: SearchFilters,
    pub rejected_filters: Vec<String>,
}

pub fn parse_search_query(input: &str) -> ParsedSearchQuery {
    let mut result = ParsedSearchQuery::default();
    let mut text = Vec::new();
    for token in tokenize(input) {
        let Some((key, raw)) = token.split_once(':') else {
            text.push(token);
            continue;
        };
        let key = key.to_ascii_lowercase();
        let handled = match key.as_str() {
            "ext" | "extension" => {
                let value = raw.trim().trim_start_matches('.').to_ascii_lowercase();
                if value.is_empty() {
                    false
                } else {
                    result.filters.extension = Some(value);
                    true
                }
            }
            "size" => match parse_range_u64(raw, parse_size) {
                Some((min, max)) => {
                    result.filters.min_size = merge_min(result.filters.min_size, min);
                    result.filters.max_size = merge_max(result.filters.max_size, max);
                    true
                }
                None => false,
            },
            "path" | "in" => {
                let value = normalize_name(raw.trim());
                if value.is_empty() {
                    false
                } else {
                    result.filters.path_contains = Some(value);
                    true
                }
            }
            "date" | "modified" | "mtime" => match parse_date_constraint(raw) {
                Some((after, before)) => {
                    result.filters.modified_after =
                        merge_min_i64(result.filters.modified_after, after);
                    result.filters.modified_before =
                        merge_max_i64(result.filters.modified_before, before);
                    true
                }
                None => false,
            },
            "type" | "kind" => match raw.to_ascii_lowercase().as_str() {
                "file" | "f" => {
                    result.filters.item_type = Some(ItemTypeFilter::File);
                    true
                }
                "dir" | "directory" | "folder" | "d" => {
                    result.filters.item_type = Some(ItemTypeFilter::Directory);
                    true
                }
                "hidden" => {
                    result.filters.item_type = Some(ItemTypeFilter::Hidden);
                    true
                }
                "system" => {
                    result.filters.item_type = Some(ItemTypeFilter::System);
                    true
                }
                _ => false,
            },
            "app" | "application" => {
                let value = normalize_name(raw.trim());
                if value.is_empty() {
                    false
                } else {
                    result.filters.application = Some(value);
                    true
                }
            }
            _ => {
                text.push(token);
                continue;
            }
        };
        if !handled {
            result.rejected_filters.push(format!("{key}:{raw}"));
        }
    }
    result.text = text.join(" ").trim().to_string();
    result
}

pub fn matches_filters(
    filters: &SearchFilters,
    name: &str,
    path: &str,
    flags: u16,
    attributes: Option<AttributeEntry>,
) -> bool {
    if let Some(extension) = &filters.extension {
        let found = name
            .rsplit_once('.')
            .map(|(_, ext)| ext)
            .unwrap_or("")
            .to_ascii_lowercase();
        if &found != extension {
            return false;
        }
    }
    if let Some(item_type) = filters.item_type {
        let matches = match item_type {
            ItemTypeFilter::File => flags & FLAG_DIRECTORY == 0,
            ItemTypeFilter::Directory => flags & FLAG_DIRECTORY != 0,
            ItemTypeFilter::Hidden => flags & FLAG_HIDDEN != 0,
            ItemTypeFilter::System => flags & FLAG_SYSTEM != 0,
        };
        if !matches {
            return false;
        }
    }
    if let Some(needle) = &filters.path_contains {
        if !normalize_name(path).contains(needle) {
            return false;
        }
    }
    if let Some(application) = &filters.application {
        let normalized_path = normalize_name(path);
        let normalized_name = normalize_name(name);
        let mut found =
            normalized_path.contains(application) || normalized_name.contains(application);
        if !found {
            if let Some(relation) = relation_for_query(application) {
                found = relation.aliases.iter().any(|alias| {
                    let alias = normalize_name(alias);
                    normalized_name == alias || normalized_path.contains(&alias)
                });
            }
        }
        if !found {
            return false;
        }
    }
    if filters.needs_attributes() {
        let Some(attributes) = attributes else {
            return false;
        };
        if filters
            .min_size
            .is_some_and(|min| attributes.size_bytes < min)
        {
            return false;
        }
        if filters
            .max_size
            .is_some_and(|max| attributes.size_bytes > max)
        {
            return false;
        }
        if filters
            .modified_after
            .is_some_and(|after| attributes.modified_unix_secs < after)
        {
            return false;
        }
        if filters
            .modified_before
            .is_some_and(|before| attributes.modified_unix_secs > before)
        {
            return false;
        }
    }
    true
}

fn tokenize(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for ch in input.chars() {
        match quote {
            Some(q) if ch == q => quote = None,
            Some(_) => current.push(ch),
            None if matches!(ch, '\'' | '"') => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_range_u64<F>(raw: &str, parse: F) -> Option<(Option<u64>, Option<u64>)>
where
    F: Fn(&str) -> Option<u64>,
{
    let raw = raw.trim();
    if let Some((left, right)) = raw.split_once("..") {
        return Some((
            (!left.is_empty()).then(|| parse(left)).flatten(),
            (!right.is_empty()).then(|| parse(right)).flatten(),
        ));
    }
    if let Some(value) = raw.strip_prefix(">=") {
        return Some((Some(parse(value)?), None));
    }
    if let Some(value) = raw.strip_prefix('>') {
        return Some((Some(parse(value)?.saturating_add(1)), None));
    }
    if let Some(value) = raw.strip_prefix("<=") {
        return Some((None, Some(parse(value)?)));
    }
    if let Some(value) = raw.strip_prefix('<') {
        return Some((None, Some(parse(value)?.saturating_sub(1))));
    }
    let value = parse(raw)?;
    Some((Some(value), Some(value)))
}

pub fn parse_size(raw: &str) -> Option<u64> {
    let raw = raw.trim().to_ascii_lowercase();
    let split = raw
        .find(|ch: char| !ch.is_ascii_digit() && ch != '.')
        .unwrap_or(raw.len());
    let value: f64 = raw[..split].parse().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let unit = raw[split..].trim();
    let multiplier = match unit {
        "" | "b" => 1_f64,
        "kb" => 1_000_f64,
        "mb" => 1_000_000_f64,
        "gb" => 1_000_000_000_f64,
        "tb" => 1_000_000_000_000_f64,
        "kib" => 1024_f64,
        "mib" => 1024_f64 * 1024_f64,
        "gib" => 1024_f64 * 1024_f64 * 1024_f64,
        "tib" => 1024_f64 * 1024_f64 * 1024_f64 * 1024_f64,
        _ => return None,
    };
    let bytes = value * multiplier;
    (bytes <= u64::MAX as f64).then_some(bytes.round() as u64)
}

fn parse_date_constraint(raw: &str) -> Option<(Option<i64>, Option<i64>)> {
    let raw = raw.trim();
    if let Some(value) = raw.strip_prefix(">=") {
        return Some((Some(parse_date(value)?), None));
    }
    if let Some(value) = raw.strip_prefix('>') {
        return Some((Some(parse_date(value)?.saturating_add(86_400)), None));
    }
    if let Some(value) = raw.strip_prefix("<=") {
        return Some((None, Some(parse_date(value)?.saturating_add(86_399))));
    }
    if let Some(value) = raw.strip_prefix('<') {
        return Some((None, Some(parse_date(value)?.saturating_sub(1))));
    }
    if let Some((left, right)) = raw.split_once("..") {
        let after = (!left.is_empty()).then(|| parse_date(left)).flatten();
        let before = (!right.is_empty())
            .then(|| parse_date(right).map(|v| v.saturating_add(86_399)))
            .flatten();
        return Some((after, before));
    }
    let day = parse_date(raw)?;
    Some((Some(day), Some(day.saturating_add(86_399))))
}

pub fn parse_date(raw: &str) -> Option<i64> {
    let mut parts = raw.trim().split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let days = days_from_civil(year, month, day)?;
    days.checked_mul(86_400)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> Option<i64> {
    let y = year - i64::from(month <= 2);
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let result = era * 146_097 + doe - 719_468;
    let (ry, rm, rd) = civil_from_days(result);
    (ry == year && rm == month && rd == day).then_some(result)
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    (y + i64::from(m <= 2), m, d)
}

fn merge_min(current: Option<u64>, next: Option<u64>) -> Option<u64> {
    match (current, next) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}
fn merge_max(current: Option<u64>, next: Option<u64>) -> Option<u64> {
    match (current, next) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}
fn merge_min_i64(current: Option<i64>, next: Option<i64>) -> Option<i64> {
    match (current, next) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (a, b) => a.or(b),
    }
}
fn merge_max_i64(current: Option<i64>, next: Option<i64>) -> Option<i64> {
    match (current, next) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_compound_filters_without_losing_text() {
        let parsed = parse_search_query(
            "node project ext:rs size:>10mb path:\"C:\\Work Space\" date:2026-01-01..2026-02-01 type:file app:rust",
        );
        assert_eq!(parsed.text, "node project");
        assert_eq!(parsed.filters.extension.as_deref(), Some("rs"));
        assert_eq!(parsed.filters.min_size, Some(10_000_001));
        assert_eq!(
            parsed.filters.path_contains.as_deref(),
            Some("c:\\work space")
        );
        assert_eq!(parsed.filters.item_type, Some(ItemTypeFilter::File));
        assert_eq!(parsed.filters.application.as_deref(), Some("rust"));
        assert!(parsed.filters.modified_after.is_some());
        assert!(parsed.filters.modified_before.is_some());
        assert!(parsed.rejected_filters.is_empty());
    }

    #[test]
    fn sizes_support_decimal_and_binary_units() {
        assert_eq!(parse_size("1MB"), Some(1_000_000));
        assert_eq!(parse_size("1MiB"), Some(1_048_576));
        assert_eq!(parse_size("1.5kb"), Some(1500));
    }

    #[test]
    fn date_conversion_rejects_impossible_dates() {
        assert_eq!(parse_date("1970-01-01"), Some(0));
        assert_eq!(parse_date("2026-02-29"), None);
        assert!(parse_date("2024-02-29").is_some());
    }

    #[test]
    fn matches_basic_filters() {
        let parsed = parse_search_query("ext:rs size:>1kb type:file");
        let attrs = AttributeEntry {
            file_id: 1,
            size_bytes: 2000,
            modified_unix_secs: 0,
            platform_attributes: 0,
        };
        assert!(matches_filters(
            &parsed.filters,
            "main.rs",
            "C:\\src\\main.rs",
            0,
            Some(attrs)
        ));
        assert!(!matches_filters(
            &parsed.filters,
            "main.txt",
            "C:\\src\\main.txt",
            0,
            Some(attrs)
        ));
    }
}
