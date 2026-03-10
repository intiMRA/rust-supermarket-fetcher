use serde::{Deserialize, Serialize};
use crate::loggers::parse_logger::log_parse_warning;

/// Simplified size unit enum with base units only.
/// All parsed values are normalized during parsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SizeUnit {
    /// Weight in kilograms (grams are converted: g ÷ 1000)
    Kilogram(f64),
    /// Volume in liters (milliliters are converted: ml ÷ 1000)
    Liter(f64),
    /// Length in meters
    Meter(f64),
    /// Length in millimeters (centimeters are converted: cm × 10)
    Millimeter(f64),
    /// Weight in milligrams (for very small quantities)
    Milligram(f64),
    /// Length in inches (imperial, kept separate)
    Inch(f64),
    /// Count-based unit (pack, piece, each, sheet, tablet, pair, serve all normalize to this)
    Unit(f64),
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

    /// Helper to create Kilogram from grams (normalizes during parsing)
    fn kilogram_from_grams(grams: f64) -> SizeUnit {
        SizeUnit::Kilogram(grams / 1000.0)
    }

    /// Helper to create Liter from milliliters (normalizes during parsing)
    fn liter_from_ml(ml: f64) -> SizeUnit {
        SizeUnit::Liter(ml / 1000.0)
    }

    /// Helper to create Millimeter from centimeters (normalizes during parsing)
    fn mm_from_cm(cm: f64) -> SizeUnit {
        SizeUnit::Millimeter(cm * 10.0)
    }

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
        let normalized_range = s.replace(Self::SEPARATOR_RANGE_SPACED, "-");

        // Handle range format: "0.55-0.75kg", "0.65-1.2kg 12pcs"
        let range_part = normalized_range.split_whitespace().next().unwrap_or(&normalized_range);
        if range_part.contains(Self::SEPARATOR_RANGE) && !range_part.starts_with(Self::SEPARATOR_RANGE) {
            if let Some(result) = Self::parse_range(range_part) {
                return result;
            }
        }

        // Handle word aliases → Unit
        match s.as_str() {
            "each" | "single" | "ea" => return SizeUnit::Unit(1.0),
            "pk" => return SizeUnit::Unit(1.0),
            _ => {}
        }

        // Handle "ea" with number prefix → Unit
        if let Some(value_str) = s.strip_suffix("ea") {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return SizeUnit::Unit(v);
            }
        }

        // Handle bare units without numbers
        if let Some(unit) = Self::parse_bare_unit(&s) {
            return unit;
        }

        // Handle count suffix "s" pattern: "100s", "45s" → Unit
        if let Some(value_str) = s.strip_suffix('s') {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return SizeUnit::Unit(v);
            }
        }

        // Standard unit parsing with numeric prefix
        if let Some(unit) = Self::parse_suffixed_unit(&s, original) {
            return unit;
        }

        // Log unrecognized formats
        log_parse_warning("SizeUnit", original, "unrecognized unit format");
        SizeUnit::Unknown
    }

    fn parse_bare_unit(s: &str) -> Option<SizeUnit> {
        match s {
            "kg" => Some(SizeUnit::Kilogram(1.0)),
            "g" | "gm" => Some(SizeUnit::Kilogram(0.001)),  // 1g = 0.001kg
            "mg" => Some(SizeUnit::Milligram(1.0)),
            "ml" => Some(SizeUnit::Liter(0.001)),  // 1ml = 0.001L
            "l" => Some(SizeUnit::Liter(1.0)),
            "m" | "mtr" => Some(SizeUnit::Meter(1.0)),
            "cm" => Some(SizeUnit::Millimeter(10.0)),  // 1cm = 10mm
            "mm" => Some(SizeUnit::Millimeter(1.0)),
            "inch" => Some(SizeUnit::Inch(1.0)),
            _ => None,
        }
    }

    fn parse_suffixed_unit(s: &str, original: &str) -> Option<SizeUnit> {
        // Sheets → Unit
        for suffix in &[" sheets", "sheets", " sheet", "sheet"] {
            if let Some(value_str) = s.strip_suffix(suffix) {
                return Some(Self::parse_to_unit(value_str, original));
            }
        }

        // Tablets/capsules → Unit
        for suffix in &[" tablets", "tablets", " tabs", "tabs", " caps", "caps"] {
            if let Some(value_str) = s.strip_suffix(suffix) {
                return Some(Self::parse_to_unit(value_str, original));
            }
        }

        // Serves → Unit
        for suffix in &[" serves", "serves", " serve", "serve"] {
            if let Some(value_str) = s.strip_suffix(suffix) {
                return Some(Self::parse_to_unit(value_str, original));
            }
        }

        // Pairs → Unit
        for suffix in &[" pair", "pair", "pr"] {
            if let Some(value_str) = s.strip_suffix(suffix) {
                return Some(Self::parse_to_unit(value_str, original));
            }
        }

        // Inch
        for suffix in &[" inch", "inch"] {
            if let Some(value_str) = s.strip_suffix(suffix) {
                return Some(Self::parse_float(value_str, original, SizeUnit::Inch));
            }
        }

        // Weight units (check kg, mg, gm before g)
        if let Some(value_str) = s.strip_suffix("kg") {
            return Some(Self::parse_float(value_str, original, SizeUnit::Kilogram));
        }
        if let Some(value_str) = s.strip_suffix("mg") {
            return Some(Self::parse_float(value_str, original, SizeUnit::Milligram));
        }
        if let Some(value_str) = s.strip_suffix("gm") {
            return Some(Self::parse_float(value_str, original, Self::kilogram_from_grams));
        }
        if let Some(value_str) = s.strip_suffix('g') {
            return Some(Self::parse_float(value_str, original, Self::kilogram_from_grams));
        }

        // Volume units (check ml before l)
        if let Some(value_str) = s.strip_suffix("ml") {
            return Some(Self::parse_float(value_str, original, Self::liter_from_ml));
        }
        if let Some(value_str) = s.strip_suffix('l') {
            return Some(Self::parse_float(value_str, original, SizeUnit::Liter));
        }

        // Length units (check cm, mm, mtr before m)
        if let Some(value_str) = s.strip_suffix("cm") {
            return Some(Self::parse_float(value_str, original, Self::mm_from_cm));
        }
        if let Some(value_str) = s.strip_suffix("mm") {
            return Some(Self::parse_float(value_str, original, SizeUnit::Millimeter));
        }
        if let Some(value_str) = s.strip_suffix("mtr") {
            return Some(Self::parse_float(value_str, original, SizeUnit::Meter));
        }
        if let Some(value_str) = s.strip_suffix('m') {
            return Some(Self::parse_float(value_str, original, SizeUnit::Meter));
        }

        // Pack/piece units → Unit
        for suffix in &["pack", "pk", "pcs", "pce", "pc"] {
            if let Some(value_str) = s.strip_suffix(suffix) {
                return Some(Self::parse_to_unit(value_str, original));
            }
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

    fn parse_to_unit(value_str: &str, original: &str) -> SizeUnit {
        match value_str.trim().parse::<f64>() {
            Ok(v) => SizeUnit::Unit(v),
            Err(_) => {
                log_parse_warning("SizeUnit", original, "invalid number for unit");
                SizeUnit::Unknown
            }
        }
    }

    /// Parse "pack with inline size" format: "6pack 330ml", "15pack 330mL"
    fn parse_pack_with_size(s: &str) -> Option<SizeUnit> {
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
        if let Some(x_pos) = s.find(Self::SEPARATOR_MULTIPACK_CHAR) {
            let left = &s[..x_pos];
            let right = &s[x_pos + 1..];

            if !left.is_empty() && left.chars().all(|c| c.is_ascii_digit()) {
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
        if let Some((first, second)) = s.split_once(Self::SEPARATOR_MULTIPACK) {
            let first_has_unit = first.chars().any(|c| c.is_alphabetic());
            let second_has_unit = second.chars().any(|c| c.is_alphabetic());

            if first_has_unit && second_has_unit {
                let result = Self::parse(first);
                if result != SizeUnit::Unknown {
                    return Some(result);
                }
            }
        }

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
        // Define range suffixes with their constructors
        let range_suffixes: &[(&str, fn(f64) -> SizeUnit)] = &[
            ("kg", SizeUnit::Kilogram),
            ("mg", SizeUnit::Milligram),
            ("gm", Self::kilogram_from_grams),
            ("g", Self::kilogram_from_grams),
            ("ml", Self::liter_from_ml),
            ("l", SizeUnit::Liter),
            ("cm", Self::mm_from_cm),
            ("mm", SizeUnit::Millimeter),
            ("m", SizeUnit::Meter),
        ];

        for (suffix, constructor) in range_suffixes {
            if let Some(range_str) = s.strip_suffix(suffix) {
                if let Some((min_str, max_str)) = range_str.split_once(Self::SEPARATOR_RANGE) {
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
    pub fn to_value_and_unit(&self) -> (f64, &'static str) {
        match self {
            SizeUnit::Kilogram(v) => (*v, "Kilogram"),
            SizeUnit::Liter(v) => (*v, "Liter"),
            SizeUnit::Meter(v) => (*v, "Meter"),
            SizeUnit::Millimeter(v) => (*v, "Millimeter"),
            SizeUnit::Milligram(v) => (*v, "Milligram"),
            SizeUnit::Inch(v) => (*v, "Inch"),
            SizeUnit::Unit(v) => (*v, "Unit"),
            SizeUnit::MultiPack { count, unit } => {
                let (inner_val, inner_unit) = unit.to_value_and_unit();
                (*count as f64 * inner_val, inner_unit)
            },
            SizeUnit::PerUnit(unit) => unit.to_value_and_unit(),
            SizeUnit::Range { min, max, unit } => {
                let (_, unit_name) = unit.to_value_and_unit();
                ((min + max) / 2.0, unit_name)
            },
            SizeUnit::Unknown => (0.0, "Unknown"),
        }
    }

    /// Extract the numeric value and unit name, normalized to base units.
    ///
    /// All sizes are already normalized during parsing:
    /// - Weight → Kilogram
    /// - Volume → Liter
    /// - Length → Meter (mm converted to m)
    /// - Count units → Unit
    pub fn to_normalized_value_and_unit(&self) -> (f64, &'static str) {
        match self {
            SizeUnit::Kilogram(v) => (*v, "Kilogram"),
            SizeUnit::Milligram(v) => (*v / 1_000_000.0, "Kilogram"),
            SizeUnit::Liter(v) => (*v, "Liter"),
            SizeUnit::Meter(v) => (*v, "Meter"),
            SizeUnit::Millimeter(v) => (*v / 1000.0, "Meter"),
            SizeUnit::Inch(v) => (*v, "Inch"),
            SizeUnit::Unit(v) => (*v, "Unit"),
            SizeUnit::MultiPack { count, unit } => {
                let (inner_val, inner_unit) = unit.to_normalized_value_and_unit();
                (*count as f64 * inner_val, inner_unit)
            },
            SizeUnit::PerUnit(unit) => unit.to_normalized_value_and_unit(),
            SizeUnit::Range { min, max, unit } => {
                let (_, unit_name) = unit.to_normalized_value_and_unit();
                ((min + max) / 2.0, unit_name)
            },
            SizeUnit::Unknown => (0.0, "Unknown"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_weight_normalizes_to_kg() {
        // Grams become kilograms
        assert_eq!(SizeUnit::parse("500g"), SizeUnit::Kilogram(0.5));
        assert_eq!(SizeUnit::parse("1000g"), SizeUnit::Kilogram(1.0));
        assert_eq!(SizeUnit::parse("250gm"), SizeUnit::Kilogram(0.25));

        // Kilograms stay as kilograms
        assert_eq!(SizeUnit::parse("1kg"), SizeUnit::Kilogram(1.0));
        assert_eq!(SizeUnit::parse("0.5kg"), SizeUnit::Kilogram(0.5));
    }

    #[test]
    fn test_parse_volume_normalizes_to_liter() {
        // Milliliters become liters
        assert_eq!(SizeUnit::parse("1000ml"), SizeUnit::Liter(1.0));
        assert_eq!(SizeUnit::parse("500ml"), SizeUnit::Liter(0.5));

        // Liters stay as liters
        assert_eq!(SizeUnit::parse("2l"), SizeUnit::Liter(2.0));
        assert_eq!(SizeUnit::parse("1.5l"), SizeUnit::Liter(1.5));
    }

    #[test]
    fn test_parse_length_normalizes_to_mm() {
        // Centimeters become millimeters
        assert_eq!(SizeUnit::parse("10cm"), SizeUnit::Millimeter(100.0));
        assert_eq!(SizeUnit::parse("5cm"), SizeUnit::Millimeter(50.0));

        // Millimeters stay as millimeters
        assert_eq!(SizeUnit::parse("100mm"), SizeUnit::Millimeter(100.0));

        // Meters stay as meters
        assert_eq!(SizeUnit::parse("2m"), SizeUnit::Meter(2.0));
    }

    #[test]
    fn test_parse_count_units_to_unit() {
        // All count-based types become Unit
        assert_eq!(SizeUnit::parse("6pack"), SizeUnit::Unit(6.0));
        assert_eq!(SizeUnit::parse("12pcs"), SizeUnit::Unit(12.0));
        assert_eq!(SizeUnit::parse("each"), SizeUnit::Unit(1.0));
        assert_eq!(SizeUnit::parse("100 sheets"), SizeUnit::Unit(100.0));
        assert_eq!(SizeUnit::parse("30 tablets"), SizeUnit::Unit(30.0));
        assert_eq!(SizeUnit::parse("2 pair"), SizeUnit::Unit(2.0));
        assert_eq!(SizeUnit::parse("4 serves"), SizeUnit::Unit(4.0));
    }

    #[test]
    fn test_same_product_normalizes_same() {
        // "500g" and "0.5kg" should both be Kilogram(0.5)
        assert_eq!(SizeUnit::parse("500g"), SizeUnit::parse("0.5kg"));

        // "1000ml" and "1l" should both be Liter(1.0)
        assert_eq!(SizeUnit::parse("1000ml"), SizeUnit::parse("1l"));
    }

    #[test]
    fn test_multipack_with_normalized_inner() {
        let parsed = SizeUnit::parse("6 x 330ml");
        match parsed {
            SizeUnit::MultiPack { count, unit } => {
                assert_eq!(count, 6);
                assert_eq!(*unit, SizeUnit::Liter(0.33));
            }
            _ => panic!("Expected MultiPack"),
        }
    }

    #[test]
    fn test_to_normalized_value_and_unit() {
        assert_eq!(SizeUnit::Kilogram(0.5).to_normalized_value_and_unit(), (0.5, "Kilogram"));
        assert_eq!(SizeUnit::Liter(1.0).to_normalized_value_and_unit(), (1.0, "Liter"));
        assert_eq!(SizeUnit::Millimeter(100.0).to_normalized_value_and_unit(), (0.1, "Meter"));
        assert_eq!(SizeUnit::Unit(6.0).to_normalized_value_and_unit(), (6.0, "Unit"));
    }
}
