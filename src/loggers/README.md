# Loggers Module

Logging utilities for debugging and monitoring.

## Files

| File | Purpose |
|------|---------|
| `logger.rs` | General purpose logger with prefix support |
| `logger_trait.rs` | Trait definition for loggers |
| `parse_logger.rs` | Specialized logger for parse warnings |

## Parse Logger

Logs unrecognized formats during data parsing to `data/parse_warnings.log`.

Used by `SizeUnit::parse()` to track patterns that need handling:

```
[SizeUnit] "6pack 330ml" - unrecognized unit format
[SizeUnit] "average 1kg" - unrecognized unit format
```

### Usage

```rust
use crate::loggers::parse_logger::log_parse_warning;

log_parse_warning("SizeUnit", original_string, "unrecognized unit format");
```

### Clearing

Log is cleared at the start of each fetch run:

```rust
clear_parse_log();
```

## General Logger

Prefixed console logging for fetch operations:

```rust
let logger = Logger::new("NewWorld");
logger.info("Fetching store 1 of 50");
// [NewWorld] Fetching store 1 of 50
```
