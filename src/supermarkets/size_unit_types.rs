use serde::{Deserialize, Serialize};
use crate::loggers::parse_logger::log_parse_warning;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SizeUnit {
    Kilogram(f64),
    Gram(f64),
    Liter(f64),
    Milliliter(f64),
    Meter(f64),
    Centimeter(f64),
    Millimeter(f64),
    Milligram(f64),
    Inch(f64),
    Pack(u32),
    Piece(u32),
    Each(u32),
    Sheet(u32),
    Tablet(u32),
    Pair(u32),
    Serve(u32),
    /// Multipack: count × unit (e.g., "6 x 250ml" = 6 packs of 250ml each)
    MultiPack { count: u32, unit: Box<SizeUnit> },
    /// Per unit pricing (e.g., "per kg")
    PerUnit(Box<SizeUnit>),
    /// Weight range for variable products (e.g., "0.55-0.75kg")
    Range { min: f64, max: f64, unit: Box<SizeUnit> },
    Unknown
}

impl SizeUnit {
    // ===== Prefixes =====
    const PREFIX_PER: &'static str = "per ";
    const PREFIX_PACK_SPACE: &'static str = "pack ";

    /// Prefixes that can be stripped before parsing the actual unit
    const IGNORABLE_PREFIXES: &'static [&'static str] = &[
        "average ",
        "min order ",
        "convenience ",
        "steak ",
        "sugar ",
    ];

    // ===== Separators =====
    const SEPARATOR_MULTIPACK: &'static str = " x ";
    const SEPARATOR_RANGE_SPACED: &'static str = " - ";
    const SEPARATOR_RANGE: char = '-';
    const SEPARATOR_MULTIPACK_CHAR: char = 'x';

    // ===== Special words =====
    const WORD_UNKNOWN: &'static str = "unknown";
    const WORD_WIDE: &'static str = "wide";

    // ===== Word aliases (exact matches) =====
    const ALIAS_EACH: &'static str = "each";
    const ALIAS_SINGLE: &'static str = "single";
    const ALIAS_EA: &'static str = "ea";
    const ALIAS_PK: &'static str = "pk";

    // ===== Unit suffixes - Weight =====
    const SUFFIX_KG: &'static str = "kg";
    const SUFFIX_MG: &'static str = "mg";
    const SUFFIX_GM: &'static str = "gm";
    const SUFFIX_G: char = 'g';

    // ===== Unit suffixes - Volume =====
    const SUFFIX_ML: &'static str = "ml";
    const SUFFIX_L: char = 'l';

    // ===== Unit suffixes - Length =====
    const SUFFIX_CM: &'static str = "cm";
    const SUFFIX_MM: &'static str = "mm";
    const SUFFIX_MTR: &'static str = "mtr";
    const SUFFIX_M: char = 'm';
    const SUFFIX_INCH: &'static str = "inch";
    const SUFFIX_INCH_SPACE: &'static str = " inch";

    // ===== Unit suffixes - Count/Pack =====
    const SUFFIX_PACK: &'static str = "pack";
    const SUFFIX_PK: &'static str = "pk";
    const SUFFIX_PCS: &'static str = "pcs";
    const SUFFIX_PCE: &'static str = "pce";
    const SUFFIX_PC: &'static str = "pc";
    const SUFFIX_EA: &'static str = "ea";
    const SUFFIX_S: char = 's';

    // ===== Unit suffixes - Sheets =====
    const SUFFIX_SHEETS_SPACE: &'static str = " sheets";
    const SUFFIX_SHEETS: &'static str = "sheets";
    const SUFFIX_SHEET_SPACE: &'static str = " sheet";
    const SUFFIX_SHEET: &'static str = "sheet";

    // ===== Unit suffixes - Tablets =====
    const SUFFIX_TABLETS_SPACE: &'static str = " tablets";
    const SUFFIX_TABLETS: &'static str = "tablets";
    const SUFFIX_TABS_SPACE: &'static str = " tabs";
    const SUFFIX_TABS: &'static str = "tabs";
    const SUFFIX_CAPS_SPACE: &'static str = " caps";
    const SUFFIX_CAPS: &'static str = "caps";

    // ===== Unit suffixes - Serves =====
    const SUFFIX_SERVES_SPACE: &'static str = " serves";
    const SUFFIX_SERVES: &'static str = "serves";
    const SUFFIX_SERVE_SPACE: &'static str = " serve";
    const SUFFIX_SERVE: &'static str = "serve";

    // ===== Unit suffixes - Pairs =====
    const SUFFIX_PAIR_SPACE: &'static str = " pair";
    const SUFFIX_PAIR: &'static str = "pair";
    const SUFFIX_PR: &'static str = "pr";

    // ===== Unit suffix arrays for range parsing =====
    const RANGE_UNIT_SUFFIXES: &'static [(&'static str, fn(f64) -> SizeUnit)] = &[
        ("kg", SizeUnit::Kilogram),
        ("mg", SizeUnit::Milligram),
        ("gm", SizeUnit::Gram),
        ("g", SizeUnit::Gram),
        ("ml", SizeUnit::Milliliter),
        ("l", SizeUnit::Liter),
        ("cm", SizeUnit::Centimeter),
        ("mm", SizeUnit::Millimeter),
        ("m", SizeUnit::Meter),
    ];

    pub fn parse(s: &str) -> SizeUnit {
        let original = s;
        let mut s = s.trim().to_lowercase();

        // Handle empty/unknown
        if s.is_empty() || s == Self::WORD_UNKNOWN {
            return SizeUnit::Unknown;
        }

        // Strip ignorable prefixes
        for prefix in Self::IGNORABLE_PREFIXES {
            if let Some(rest) = s.strip_prefix(prefix) {
                s = rest.to_string();
                break;
            }
        }

        // Handle "per kg", "per g", etc.
        if let Some(rest) = s.strip_prefix(Self::PREFIX_PER) {
            let inner = Self::parse(rest);
            if inner != SizeUnit::Unknown {
                return SizeUnit::PerUnit(Box::new(inner));
            }
        }

        // Handle "pack with inline size" format: "6pack 330ml", "15pack 330mL"
        if let Some(result) = Self::parse_pack_with_size(&s) {
            return result;
        }

        // Handle multipack format: "6 x 250ml", "10 x 17g"
        if let Some((count_str, unit_str)) = s.split_once(Self::SEPARATOR_MULTIPACK) {
            if let Ok(count) = count_str.trim().parse::<u32>() {
                let inner = Self::parse(unit_str);
                if inner != SizeUnit::Unknown {
                    return SizeUnit::MultiPack { count, unit: Box::new(inner) };
                }
            }
        }

        // Handle multipack without space: "10x22g", "6x330ml"
        if let Some(result) = Self::parse_multipack_no_space(&s) {
            return result;
        }

        // Handle dimension format: "12mm x 15m" - parse first dimension only
        if let Some(result) = Self::parse_dimension(&s) {
            return result;
        }

        // Handle range format with spaces: "0.5 - 0.7kg", "2.5 - 3.5kg"
        // Normalize by removing spaces around dash
        let normalized_range = s.replace(Self::SEPARATOR_RANGE_SPACED, "-");

        // Handle range format: "0.55-0.75kg", "0.65-1.2kg 12pcs"
        // Take first part before space for ranges with extra info
        let range_part = normalized_range.split_whitespace().next().unwrap_or(&normalized_range);
        if range_part.contains(Self::SEPARATOR_RANGE) && !range_part.starts_with(Self::SEPARATOR_RANGE) {
            if let Some(result) = Self::parse_range(range_part) {
                return result;
            }
        }

        // Handle word aliases
        match s.as_str() {
            Self::ALIAS_EACH | Self::ALIAS_SINGLE => return SizeUnit::Each(1),
            Self::ALIAS_PK => return SizeUnit::Pack(1),
            Self::ALIAS_EA => return SizeUnit::Each(1),
            _ => {}
        }

        // Handle "ea" with number prefix
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_EA) {
            if let Ok(v) = value_str.trim().parse::<u32>() {
                return SizeUnit::Each(v);
            }
        }

        // Handle bare units without numbers
        if let Some(unit) = Self::parse_bare_unit(&s) {
            return unit;
        }

        // Handle count suffix "s" pattern: "100s", "45s" (for sheets/tablets context)
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_S) {
            // Only if it's pure digits before 's'
            if let Ok(v) = value_str.trim().parse::<u32>() {
                // Ambiguous - could be sheets, tablets, etc. Default to Piece
                return SizeUnit::Piece(v);
            }
        }

        // Standard unit parsing with numeric prefix
        // Order matters: check longer suffixes first
        if let Some(unit) = Self::parse_suffixed_unit(&s, original) {
            return unit;
        }

        // Log unrecognized formats
        log_parse_warning("SizeUnit", original, "unrecognized unit format");
        SizeUnit::Unknown
    }

    fn parse_bare_unit(s: &str) -> Option<SizeUnit> {
        match s {
            Self::SUFFIX_KG => Some(SizeUnit::Kilogram(1.0)),
            "g" | Self::SUFFIX_GM => Some(SizeUnit::Gram(1.0)),
            "mg" => Some(SizeUnit::Milligram(1.0)),
            "ml" => Some(SizeUnit::Milliliter(1.0)),
            "l" => Some(SizeUnit::Liter(1.0)),
            "m" | Self::SUFFIX_MTR => Some(SizeUnit::Meter(1.0)),
            "cm" => Some(SizeUnit::Centimeter(1.0)),
            "mm" => Some(SizeUnit::Millimeter(1.0)),
            Self::SUFFIX_INCH => Some(SizeUnit::Inch(1.0)),
            _ => None,
        }
    }

    fn parse_suffixed_unit(s: &str, original: &str) -> Option<SizeUnit> {
        // Sheets
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_SHEETS_SPACE) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Sheet));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_SHEETS) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Sheet));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_SHEET_SPACE) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Sheet));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_SHEET) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Sheet));
        }

        // Tablets/capsules
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_TABLETS_SPACE) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Tablet));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_TABLETS) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Tablet));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_TABS_SPACE) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Tablet));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_TABS) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Tablet));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_CAPS_SPACE) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Tablet));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_CAPS) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Tablet));
        }

        // Serves
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_SERVES_SPACE) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Serve));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_SERVES) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Serve));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_SERVE_SPACE) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Serve));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_SERVE) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Serve));
        }

        // Pairs
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_PAIR_SPACE) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Pair));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_PAIR) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Pair));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_PR) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Pair));
        }

        // Inch
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_INCH_SPACE) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Inch));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_INCH) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Inch));
        }

        // Weight units (check kg, mg, gm before g)
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_KG) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Kilogram));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_MG) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Milligram));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_GM) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Gram));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_G) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Gram));
        }

        // Volume units (check ml before l)
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_ML) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Milliliter));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_L) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Liter));
        }

        // Length units (check cm, mm, mtr before m)
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_CM) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Centimeter));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_MM) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Millimeter));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_MTR) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Meter));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_M) {
            return Some(Self::parse_float(value_str, original, SizeUnit::Meter));
        }

        // Pack/piece units
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_PACK) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Pack));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_PK) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Pack));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_PCS) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Piece));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_PCE) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Piece));
        }
        if let Some(value_str) = s.strip_suffix(Self::SUFFIX_PC) {
            return Some(Self::parse_int(value_str, original, SizeUnit::Piece));
        }

        None
    }

    fn parse_float<F>(value_str: &str, original: &str, constructor: F) -> SizeUnit
    where
        F: FnOnce(f64) -> SizeUnit,
    {
        match value_str.trim().parse::<f64>() {
            Ok(v) => constructor(v),
            Err(_) => {
                log_parse_warning("SizeUnit", original, "invalid number for float unit");
                SizeUnit::Unknown
            }
        }
    }

    fn parse_int<F>(value_str: &str, original: &str, constructor: F) -> SizeUnit
    where
        F: FnOnce(u32) -> SizeUnit,
    {
        match value_str.trim().parse::<u32>() {
            Ok(v) => constructor(v),
            Err(_) => {
                log_parse_warning("SizeUnit", original, "invalid number for integer unit");
                SizeUnit::Unknown
            }
        }
    }

    /// Parse "pack with inline size" format: "6pack 330ml", "15pack 330mL"
    fn parse_pack_with_size(s: &str) -> Option<SizeUnit> {
        // Look for pattern: NUMBERpack UNIT
        if let Some(pack_pos) = s.find(Self::PREFIX_PACK_SPACE) {
            let count_str = &s[..pack_pos];
            let unit_str = &s[pack_pos + Self::PREFIX_PACK_SPACE.len()..];

            if let Ok(count) = count_str.trim().parse::<u32>() {
                let inner = Self::parse(unit_str);
                if inner != SizeUnit::Unknown {
                    return Some(SizeUnit::MultiPack { count, unit: Box::new(inner) });
                }
            }
        }
        None
    }

    /// Parse multipack without space: "10x22g", "6x330ml"
    fn parse_multipack_no_space(s: &str) -> Option<SizeUnit> {
        // Find 'x' that's between digits
        if let Some(x_pos) = s.find(Self::SEPARATOR_MULTIPACK_CHAR) {
            let left = &s[..x_pos];
            let right = &s[x_pos + 1..];

            // Left must be all digits (the count)
            if !left.is_empty() && left.chars().all(|c| c.is_ascii_digit()) {
                // Right must start with a digit (the unit value)
                if !right.is_empty() && right.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    if let Ok(count) = left.parse::<u32>() {
                        let inner = Self::parse(right);
                        if inner != SizeUnit::Unknown {
                            return Some(SizeUnit::MultiPack { count, unit: Box::new(inner) });
                        }
                    }
                }
            }
        }
        None
    }

    /// Parse dimension format: "12mm x 15m" - returns first dimension only
    fn parse_dimension(s: &str) -> Option<SizeUnit> {
        // Look for "X x Y" pattern where X and Y are both units (not just numbers)
        // This handles cases like "12mm x 15m", "150mm x 68mm"
        if let Some((first, second)) = s.split_once(Self::SEPARATOR_MULTIPACK) {
            // Check if this looks like a dimension (both parts have units)
            let first_has_unit = first.chars().any(|c| c.is_alphabetic());
            let second_has_unit = second.chars().any(|c| c.is_alphabetic());

            if first_has_unit && second_has_unit {
                // Parse just the first dimension
                let result = Self::parse(first);
                if result != SizeUnit::Unknown {
                    return Some(result);
                }
            }
        }

        // Also handle "10m x 30cm wide" type patterns
        if s.contains(Self::SEPARATOR_MULTIPACK) && s.contains(Self::WORD_WIDE) {
            if let Some((first, _)) = s.split_once(Self::SEPARATOR_MULTIPACK) {
                let result = Self::parse(first);
                if result != SizeUnit::Unknown {
                    return Some(result);
                }
            }
        }

        None
    }

    /// Parse a range format like "0.55-0.75kg" or "0.55kg-0.7kg"
    fn parse_range(s: &str) -> Option<SizeUnit> {
        for (suffix, constructor) in Self::RANGE_UNIT_SUFFIXES {
            if let Some(range_str) = s.strip_suffix(suffix) {
                if let Some((min_str, max_str)) = range_str.split_once(Self::SEPARATOR_RANGE) {
                    // Handle case where unit appears on both sides: "0.55kg-0.7"
                    // Strip the unit from min_str if present
                    let min_clean = min_str.strip_suffix(suffix).unwrap_or(min_str);

                    if let (Ok(min), Ok(max)) = (min_clean.parse::<f64>(), max_str.parse::<f64>()) {
                        return Some(SizeUnit::Range {
                            min,
                            max,
                            unit: Box::new(constructor(1.0)),
                        });
                    }
                }
            }
        }

        None
    }

    /// Extract the numeric value and unit name as separate parts.
    ///
    /// Useful for storing in database columns.
    /// For complex types (MultiPack, PerUnit), returns the total value and a descriptive unit.
    ///
    /// # Returns
    /// A tuple of (value, unit_name) where both are strings.
    pub fn to_value_and_unit(&self) -> (f64, &str) {
        match self {
            SizeUnit::Kilogram(v) => (*v, "Kilogram"),
            SizeUnit::Gram(v) => (*v, "Gram"),
            SizeUnit::Liter(v) => (*v, "Liter"),
            SizeUnit::Milliliter(v) => (*v, "Milliliter"),
            SizeUnit::Meter(v) => (*v, "Meter"),
            SizeUnit::Centimeter(v) => (*v, "Centimeter"),
            // Normalize mm to cm (10mm = 1cm)
            SizeUnit::Millimeter(v) => (*v / 10.0, "Centimeter"),
            // Normalize mg to g (1000mg = 1g)
            SizeUnit::Milligram(v) => (*v / 1000.0, "Gram"),
            SizeUnit::Inch(v) => (*v, "Inch"),
            SizeUnit::Pack(v) => (*v as f64, "Pack"),
            SizeUnit::Piece(v) => (*v as f64, "Piece"),
            SizeUnit::Each(v) => (*v as f64, "Each"),
            SizeUnit::Sheet(v) => (*v as f64, "Sheet"),
            SizeUnit::Tablet(v) => (*v as f64, "Tablet"),
            SizeUnit::Pair(v) => (*v as f64, "Pair"),
            SizeUnit::Serve(v) => (*v as f64, "Serve"),
            SizeUnit::MultiPack { count, unit } => {
                // Return total quantity (count × inner value)
                let (inner_val, inner_unit) = unit.to_value_and_unit();
                (*count as f64 * inner_val, inner_unit)
            },
            SizeUnit::PerUnit(unit) => {
                // Return the inner unit's value
                unit.to_value_and_unit()
            },
            SizeUnit::Range { min, max, unit } => {
                // Return average of range
                let (_, unit_name) = unit.to_value_and_unit();
                ((min + max) / 2.0, unit_name)
            },
            SizeUnit::Unknown => (0.0, "Unknown"),
        }
    }
}
