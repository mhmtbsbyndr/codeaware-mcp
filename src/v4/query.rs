//! Deterministic goal normalizer for retrieval. No LLM, no fuzzy matching.
//!
//! A goal (German or English, free text) becomes two lists:
//! - `terms`: lowercased, identifier-split words without stopwords. German words that appear
//!   in [`DE_EN`] are replaced by their English development vocabulary, so a German goal
//!   queries the same terms as its English paraphrase.
//! - `identifiers`: tokens that look like code (snake_case, CamelCase, `::` paths, `/` paths,
//!   source file names). They keep their original spelling.
//!
//! Dictionary and stopword lookups compare folded words: umlauts and accents are reduced to
//! their base letter and the transliterations `ae`, `oe`, `ue` collapse too, so `Abhängigkeit`,
//! `Abhaengigkeit` and `abhangigkeit` are the same key. Unknown words are kept as typed.
//! German compounds are only translated when the exact compound is in the dictionary.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::v4::bm25::split_identifiers;

/// Upper bound for `terms` and, separately, for `identifiers`.
pub const MAX_TERMS: usize = 64;
pub const MAX_IDENTIFIERS: usize = 64;

/// Source-file extensions that mark a token as a path rather than prose.
const SOURCE_EXTENSIONS: &[&str] = &[
    "rs", "toml", "md", "json", "ts", "tsx", "js", "jsx", "py", "go", "yaml", "yml", "sh", "c", "h", "cpp",
    "java", "kt", "swift",
];

/// German stopwords and filler verbs. Matched after folding.
const DE_STOPWORDS: &[&str] = &[
    "der", "die", "das", "des", "dem", "den", "ein", "eine", "einen", "einem", "einer", "eines", "und", "oder",
    "aber", "auch", "mit", "ohne", "für", "von", "vom", "zu", "zum", "zur", "im", "in", "am", "an", "auf", "aus",
    "bei", "nach", "über", "unter", "damit", "dass", "wenn", "falls", "dann", "dabei", "sowie", "sodass", "um",
    "ob", "als", "wie", "was", "wo", "wann", "warum", "ist", "sind", "sein", "seine", "seiner", "bitte", "mal",
    "noch", "nur", "schon", "sehr", "soll", "sollte", "sollen", "muss", "müssen", "kann", "können", "wir", "ich",
    "du", "er", "sie", "es", "man", "nicht", "kein", "keine", "keinen", "alle", "alles", "jede", "jeder", "jedes",
    "machen", "mach", "mache", "macht", "hinzufügen", "beheben", "verwenden", "nutzen", "sicherstellen", "also",
    "dort", "hier", "diese", "dieser", "dieses", "diesen", "jetzt", "ganz", "etwa", "bzw",
];

/// English stopwords and filler verbs. Matched after folding.
const EN_STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "of", "to", "in", "on", "at", "by", "for", "with", "from", "into",
    "onto", "about", "as", "is", "are", "was", "were", "be", "been", "being", "it", "its", "this", "that",
    "these", "those", "we", "you", "our", "their", "they", "them", "he", "she", "i", "my", "your", "so", "if",
    "then", "than", "when", "where", "how", "what", "why", "which", "who", "not", "no", "do", "does", "did",
    "done", "can", "could", "should", "would", "will", "shall", "may", "might", "must", "make", "makes", "add",
    "fix", "use", "uses", "please", "need", "needs", "want", "wants", "ensure", "implement", "also", "just",
    "only", "more", "most", "some", "any", "all", "each", "every", "via", "per", "etc", "there", "here", "while",
    "such", "both", "either", "neither",
];

/// Compact German to English development vocabulary, grouped by topic. Keys are lowercase
/// German as typed (umlauts allowed); values are English words, several separated by spaces.
/// Compounds only match when the exact compound is listed here.
pub const DE_EN: &[(&str, &str)] = &[
    // errors and failures
    ("fehler", "error"),
    ("fehlermeldung", "error message"),
    ("fehlerbehandlung", "error handling"),
    ("fehlschlag", "failure"),
    ("absturz", "crash"),
    ("abbruch", "abort"),
    ("problem", "issue"),
    // files, paths, directories
    ("datei", "file"),
    ("dateien", "file"),
    ("ordner", "directory"),
    ("verzeichnis", "directory"),
    ("pfad", "path"),
    ("pfade", "path"),
    ("zeile", "line"),
    ("zeilen", "line"),
    // code entities
    ("funktion", "function"),
    ("funktionen", "function"),
    ("methode", "method"),
    ("klasse", "class"),
    ("struktur", "struct"),
    ("typ", "type"),
    ("typen", "type"),
    ("variable", "variable"),
    ("parameter", "parameter"),
    ("argument", "argument"),
    ("rückgabe", "return"),
    ("rückgabewert", "return value"),
    ("symbol", "symbol"),
    ("symbole", "symbol"),
    ("modul", "module"),
    ("module", "module"),
    ("schnittstelle", "interface"),
    ("schnittstellen", "interface"),
    ("hook", "hook"),
    ("hooks", "hook"),
    ("aufruf", "call"),
    ("aufrufe", "call"),
    ("aufrufer", "caller"),
    ("abhängigkeit", "dependency"),
    ("abhängigkeiten", "dependency"),
    ("import", "import"),
    ("export", "export"),
    ("exportieren", "export"),
    ("importieren", "import"),
    // tests
    ("test", "test"),
    ("tests", "test"),
    ("testfall", "test case"),
    ("testfälle", "test case"),
    // actions
    ("suchen", "search find"),
    ("suche", "search"),
    ("finden", "find"),
    ("lesen", "read"),
    ("schreiben", "write"),
    ("speichern", "save"),
    ("laden", "load"),
    ("umbenennen", "rename"),
    ("umbenennung", "rename"),
    ("refactoring", "refactor"),
    ("umstrukturieren", "refactor"),
    ("ändern", "change"),
    ("änderung", "change"),
    ("änderungen", "change"),
    ("unterschied", "diff"),
    ("filtern", "filter"),
    ("sortieren", "sort"),
    ("ausgabe", "output"),
    ("eingabe", "input"),
    ("ausführen", "run execute"),
    ("ausführung", "execution"),
    ("starten", "start"),
    ("kompilieren", "compile"),
    ("bauen", "build"),
    ("zeigen", "show"),
    ("anzeige", "display"),
    ("anzeigen", "show display"),
    ("kommentar", "comment"),
    ("kommentare", "comment"),
    // configuration, state, storage
    ("konfiguration", "config"),
    ("konfig", "config"),
    ("einstellung", "setting"),
    ("einstellungen", "setting config"),
    ("speicher", "memory"),
    ("zwischenspeicher", "cache"),
    ("datenbank", "database"),
    ("protokoll", "log"),
    ("sitzung", "session"),
    ("berechtigung", "permission"),
    ("berechtigungen", "permission"),
    ("zugriff", "access"),
    ("sicherheit", "security"),
    // context, retrieval, tasks
    ("kontext", "context"),
    ("zusammenfassung", "summary"),
    ("übersicht", "overview"),
    ("kandidat", "candidate"),
    ("kandidaten", "candidate"),
    ("relevant", "relevant"),
    ("relevanz", "relevance"),
    ("abfrage", "query"),
    ("anfrage", "request"),
    ("antwort", "response"),
    ("ergebnis", "result"),
    ("ergebnisse", "result"),
    ("aufgabe", "task"),
    ("aufgaben", "task"),
    ("ziel", "goal"),
    ("dokumentation", "documentation"),
    // services and networking
    ("server", "server"),
    ("client", "client"),
    ("verbindung", "connection"),
    ("netzwerk", "network"),
    ("nachricht", "message"),
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NormalizedGoal {
    /// Lowercased, identifier-split, stopword-free terms in first-seen order (at most `MAX_TERMS`).
    pub terms: Vec<String>,
    /// Code-like tokens with their original spelling, in first-seen order (at most `MAX_IDENTIFIERS`).
    pub identifiers: Vec<String>,
}

impl NormalizedGoal {
    /// True when the goal names code identifiers or paths. Those spellings should be matched
    /// verbatim (`identifiers`) and not only through their split terms.
    pub fn keep_exact(&self) -> bool {
        !self.identifiers.is_empty()
    }

    /// Lowercase substrings for filtering: `terms`, then the lowercased identifiers.
    pub fn match_terms(&self) -> Vec<String> {
        let mut out = self.terms.clone();
        for identifier in &self.identifiers {
            let lower = identifier.to_lowercase();
            if !out.contains(&lower) {
                out.push(lower);
            }
        }
        out
    }

    /// Space-separated query for BM25 and the substring rankers.
    pub fn search_query(&self) -> String {
        self.match_terms().join(" ")
    }
}

/// Turns a free-text goal (German or English) into retrieval terms and code identifiers.
pub fn normalize_goal(goal: &str) -> NormalizedGoal {
    let mut out = NormalizedGoal::default();
    for raw in goal.split(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | ':' | '/' | '.'))) {
        if out.terms.len() >= MAX_TERMS && out.identifiers.len() >= MAX_IDENTIFIERS {
            break;
        }
        let token = raw.trim_matches(|ch: char| matches!(ch, '_' | ':' | '/' | '.'));
        if token.is_empty() {
            continue;
        }
        if looks_like_identifier(token) && out.identifiers.len() < MAX_IDENTIFIERS && !out.identifiers.iter().any(|i| i == token) {
            out.identifiers.push(token.to_string());
        }
        for part in split_identifiers(token) {
            push_word(&mut out.terms, &part);
        }
    }
    out
}

fn push_word(terms: &mut Vec<String>, word: &str) {
    if word.chars().count() < 2 || word.chars().all(|ch| ch.is_ascii_digit()) {
        return;
    }
    let tables = tables();
    let key = fold(word);
    if tables.stopwords.contains(&key) {
        return;
    }
    match tables.dictionary.get(&key) {
        Some(english) => {
            for word in english.split_whitespace() {
                push_unique(terms, word);
            }
        }
        None => push_unique(terms, word),
    }
}

fn push_unique(terms: &mut Vec<String>, word: &str) {
    if terms.len() < MAX_TERMS && !terms.iter().any(|t| t == word) {
        terms.push(word.to_string());
    }
}

/// Code-like: a source path, a `::` or `/` path, a source file name, snake_case, or CamelCase.
fn looks_like_identifier(token: &str) -> bool {
    if token.contains("::") || token.contains('/') || has_source_extension(token) {
        return true;
    }
    let chars: Vec<char> = token.chars().collect();
    let core_has_letter = chars.iter().any(|ch| ch.is_alphabetic());
    if token.contains('_') && core_has_letter {
        return true;
    }
    let lower_then_upper = chars.windows(2).any(|w| w[0].is_lowercase() && w[1].is_uppercase());
    let acronym_then_word = chars
        .windows(3)
        .any(|w| w[0].is_uppercase() && w[1].is_uppercase() && w[2].is_lowercase());
    lower_then_upper || acronym_then_word
}

fn has_source_extension(token: &str) -> bool {
    match token.rsplit_once('.') {
        Some((stem, ext)) => {
            !stem.is_empty() && SOURCE_EXTENSIONS.contains(&ext.to_lowercase().as_str())
        }
        None => false,
    }
}

/// Folding key: lowercase, umlauts and accents to base letters, `ß` to `ss`, and the
/// transliterations `ae`, `oe`, `ue` to `a`, `o`, `u`.
fn fold(word: &str) -> String {
    let mut base = String::with_capacity(word.len());
    for ch in word.to_lowercase().chars() {
        match ch {
            'ä' | 'à' | 'á' | 'â' => base.push('a'),
            'ö' | 'ò' | 'ó' | 'ô' => base.push('o'),
            'ü' | 'ù' | 'ú' | 'û' => base.push('u'),
            'è' | 'é' | 'ê' | 'ë' => base.push('e'),
            'ì' | 'í' | 'î' => base.push('i'),
            'ç' => base.push('c'),
            'ß' => base.push_str("ss"),
            other => base.push(other),
        }
    }
    let chars: Vec<char> = base.chars().collect();
    let mut folded = String::with_capacity(base.len());
    let mut i = 0;
    while i < chars.len() {
        let collapses = i + 1 < chars.len() && chars[i + 1] == 'e' && matches!(chars[i], 'a' | 'o' | 'u');
        folded.push(chars[i]);
        i += if collapses { 2 } else { 1 };
    }
    folded
}

struct Tables {
    stopwords: HashSet<String>,
    dictionary: HashMap<String, &'static str>,
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| Tables {
        stopwords: DE_STOPWORDS.iter().chain(EN_STOPWORDS).map(|w| fold(w)).collect(),
        dictionary: DE_EN.iter().map(|(de, en)| (fold(de), *en)).collect(),
    })
}
