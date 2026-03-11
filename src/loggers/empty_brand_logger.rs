use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Mutex;

/// File path for empty brand log.
const LOG_FILE: &str = "data/empty_brand.log";

/// Global state for deduplication and thread safety.
static LOGGED_ITEMS: Mutex<Option<HashSet<String>>> = Mutex::new(None);

/// Log an item with empty brand to file (deduplicated by item id + supermarket).
pub fn log_empty_brand(item_json: &str, item_id: &str, supermarket: &str) {
    let mut guard = LOGGED_ITEMS.lock().unwrap_or_else(|e| e.into_inner());

    // Initialize the set if needed
    let seen = guard.get_or_insert_with(HashSet::new);

    // Create a unique key for this item
    let key = format!("{}|{}", supermarket, item_id);

    // Skip if already logged
    if seen.contains(&key) {
        return;
    }
    seen.insert(key);

    // Ensure data directory exists
    let _ = std::fs::create_dir_all("data");

    let log_line = format!("{}\n---\n", item_json);

    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
    {
        let _ = file.write_all(log_line.as_bytes());
    }
}

/// Clear the empty brand log file and reset deduplication.
///
/// Call this at the start of a fetch run to get fresh logs.
pub fn clear_empty_brand_log() {
    let mut guard = LOGGED_ITEMS.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(HashSet::new());

    let _ = std::fs::create_dir_all("data");
    let _ = std::fs::write(LOG_FILE, "");
}
