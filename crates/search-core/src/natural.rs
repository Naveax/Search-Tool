use crate::ai::QueryIntent;
use crate::relationship::relation_for_query;
use crate::store::normalize_name;

const STOPWORDS: &[&str] = &[
    "ara",
    "arat",
    "bul",
    "bulur",
    "bulabilir",
    "dosya",
    "dosyalar",
    "dosyaları",
    "dosyayı",
    "dosyalari",
    "içinde",
    "icinde",
    "geçen",
    "gecen",
    "olan",
    "ile",
    "ilgili",
    "alakalı",
    "alakali",
    "her",
    "şey",
    "sey",
    "şeyi",
    "seyi",
    "göster",
    "goster",
    "nerede",
    "search",
    "find",
    "file",
    "files",
    "inside",
    "contains",
    "containing",
    "related",
    "everything",
    "show",
    "the",
    "a",
    "an",
    "for",
    "content",
    "code",
    "source",
    "text",
    "metin",
    "kod",
    "adı",
    "adi",
    "isimli",
    "name",
    "named",
    "yanlış",
    "yanlis",
    "yazdım",
    "yazdim",
    "olabilir",
    "benzer",
    "fuzzy",
    "exact",
    "yaklaşık",
    "yaklasik",
    "eslesme",
    "eşleşme",
    "yap",
    "bana",
    "bunu",
    "şunu",
    "sunu",
];

pub fn rule_intent(query: &str) -> Option<QueryIntent> {
    let normalized = normalize_name(query);
    let has = |markers: &[&str]| markers.iter().any(|marker| normalized.contains(marker));

    if has(&["yardım", "yardim", "help", "nasıl kullan", "nasil kullan"]) {
        return Some(QueryIntent::Help);
    }
    // Explicit web/research language wins over content words such as "search".
    if has(&[
        "internette",
        "internetten",
        "search web",
        "web search",
        "internet search",
        "internette araştır",
        "internette arastir",
        "webde araştır",
        "webde arastir",
        "ne işe yarıyor",
        "ne ise yariyor",
        "what is ",
        "look up online",
    ]) {
        return Some(QueryIntent::WebLookup);
    }
    // Typo language wins over cleanup tokens. A misspelled node_modules query must not
    // become a delete recommendation simply because "modules" resembles dev clutter.
    if has(&[
        "yanlış yaz",
        "yanlis yaz",
        "benzer",
        "yaklaşık",
        "yaklasik",
        "fuzzy",
        "typo",
        "approximate",
        "misspell",
        "olabilir",
    ]) {
        return Some(QueryIntent::FuzzySearch);
    }
    if has(&[
        "içinde",
        "icinde",
        "geçen",
        "gecen",
        "contains",
        "containing",
        "content",
        "kodunda",
        "metninde",
        "text inside",
        "source contains",
    ]) {
        return Some(QueryIntent::ContentSearch);
    }
    if has(&[
        "gereksiz",
        "temizle",
        "cleanup",
        "clean up",
        "temporary",
        "temp files",
        "orphan",
        "duplicate",
        "cache dos",
        "cache file",
        "eski build",
        "old build",
    ]) {
        return Some(QueryIntent::CleanupAnalysis);
    }
    if relation_for_query(query).is_some()
        || has(&[
            "ile ilgili",
            "ilgili her",
            "alakalı",
            "alakali",
            "related to",
            "everything related",
            "everything about",
        ])
    {
        return Some(QueryIntent::RelatedSearch);
    }

    // A concrete filename/path in an ordinary find request is deterministic enough that
    // consulting the model only adds latency and opportunities for hallucinated intent.
    let subject = query_subject(query);
    if subject.contains('.') || subject.contains('\\') || subject.contains('/') {
        return Some(QueryIntent::ExactSearch);
    }
    None
}

pub fn query_subject(query: &str) -> String {
    if let Some(relation) = relation_for_query(query) {
        return relation.canonical.to_string();
    }
    let tokens = meaningful_tokens(query);
    if let Some(token) = tokens
        .iter()
        .find(|t| t.contains('.') || t.contains('\\') || t.contains('/'))
    {
        return (*token).to_string();
    }
    tokens
        .into_iter()
        .max_by_key(|token| token.chars().count())
        .map(str::to_string)
        .unwrap_or_else(|| normalize_name(query))
}

pub fn content_terms(query: &str) -> String {
    meaningful_tokens(query).join(" ")
}

fn meaningful_tokens(query: &str) -> Vec<&str> {
    query
        .split(|c: char| {
            c.is_whitespace() || matches!(c, ',' | ';' | ':' | '"' | '\'' | '(' | ')' | '[' | ']')
        })
        .map(str::trim)
        .filter(|token| token.chars().count() >= 2)
        .filter(|token| {
            let n = normalize_name(token);
            !STOPWORDS.contains(&n.as_str())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extracts_filename_and_content_subjects() {
        assert_eq!(query_subject("adı node.exe olan dosyayı bul"), "node.exe");
        assert_eq!(
            content_terms("içinde websocket error geçen dosyaları bul"),
            "websocket error"
        );
    }
    #[test]
    fn related_queries_canonicalize() {
        assert_eq!(query_subject("node js ile alakalı her şeyi bul"), "node.js");
    }

    #[test]
    fn deterministic_intents_cover_high_confidence_language() {
        assert_eq!(
            rule_intent("report_final.pdf dosyasını bul"),
            Some(QueryIntent::ExactSearch)
        );
        assert_eq!(
            rule_intent("kodunda TODO geçen dosyalar"),
            Some(QueryIntent::ContentSearch)
        );
        assert_eq!(
            rule_intent("what is strangecodec.dll search web"),
            Some(QueryIntent::WebLookup)
        );
        assert_eq!(
            rule_intent("yanlış yazmış olabilirim nod_modules bul"),
            Some(QueryIntent::FuzzySearch)
        );
        assert_eq!(
            rule_intent("gereksiz npm cache dosyalarını bul"),
            Some(QueryIntent::CleanupAnalysis)
        );
        assert_eq!(
            rule_intent("find files containing authentication failed"),
            Some(QueryIntent::ContentSearch)
        );
    }
}
