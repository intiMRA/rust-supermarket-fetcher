use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

/// File path for parse warnings log.
const LOG_FILE: &str = "data/parse_warnings.log";

/// Global state for deduplication and thread safety.
static LOGGED_WARNINGS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Log a parse warning to file (deduplicated).
///
/// Used when parsing fails for a value that might need to be handled.
/// Each unique (context, value, reason) combination is only logged once per run.
pub fn log_parse_warning(context: &str, value: &str, reason: &str) {
    let mut guard = LOGGED_WARNINGS.lock().unwrap_or_else(|e| e.into_inner());

    // Initialize the set if needed
    let seen = guard.get_or_insert_with(HashSet::new);

    // Create a unique key for this warning
    let key = format!("{}|{}|{}", context, value, reason);

    // Skip if already logged
    if seen.contains(&key) {
        return;
    }
    seen.insert(key);

    // Ensure data directory exists
    let _ = std::fs::create_dir_all("data");

    let log_line = format!(
        "[{}] Failed to parse: \"{}\" - {}\n",
        context, value, reason
    );

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
    {
        let _ = file.write_all(log_line.as_bytes());
    }
}

/// Clear the parse warnings log file and reset deduplication.
///
/// Call this at the start of a fetch run to get fresh logs.
pub fn clear_parse_log() {
    let mut guard = LOGGED_WARNINGS.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(HashSet::new());

    let _ = std::fs::create_dir_all("data");
    let _ = std::fs::write(LOG_FILE, "");
}
