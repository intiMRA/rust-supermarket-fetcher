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
    /// Prefixes that can be stripped before parsing
    const IGNORABLE_PREFIXES: &'static [&'static str] = &[
        "average ", "min order ", "convenience ", "steak ", "sugar ",
    ];

    /// Suffixes that can be stripped (container words)
    const IGNORABLE_SUFFIXES: &'static [&'static str] = &[
        " pks", " packs", " pouches", " bars", " pack", " roll", " tray",
        " bags", " sachets", " bottles", " cans", " tins", " jars",
    ];

    /// Helper to create Kilogram from grams
    fn kilogram_from_grams(grams: f64) -> SizeUnit {
        SizeUnit::Kilogram(grams / 1000.0)
    }

    /// Helper to create Liter from milliliters
    fn liter_from_ml(ml: f64) -> SizeUnit {
        SizeUnit::Liter(ml / 1000.0)
    }

    /// Helper to create Millimeter from centimeters
    fn mm_from_cm(cm: f64) -> SizeUnit {
        SizeUnit::Millimeter(cm * 10.0)
    }

    pub fn parse(s: &str) -> SizeUnit {
        let original = s;
        let s = s.trim().to_lowercase();

        // Handle empty/unknown
        if s.is_empty() || s == "unknown" {
            return SizeUnit::Unknown;
        }

        // Normalize: collapse multiple spaces, trim
        let s = Self::normalize_spaces(&s);

        // Strip ignorable prefixes
        let s = Self::strip_prefixes(&s);

        // Strip ignorable suffixes (container words like "pks", "packs", "pouches")
        let s = Self::strip_suffixes(&s);

        // Try all parse strategies in order
        Self::try_parse(&s, original)
    }

    fn normalize_spaces(s: &str) -> String {
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn strip_prefixes(s: &str) -> String {
        let mut result = s.to_string();
        for prefix in Self::IGNORABLE_PREFIXES {
            if let Some(rest) = result.strip_prefix(prefix) {
                result = rest.to_string();
                break;
            }
        }
        result
    }

    fn strip_suffixes(s: &str) -> String {
        let mut result = s.to_string();
        for suffix in Self::IGNORABLE_SUFFIXES {
            if let Some(rest) = result.strip_suffix(suffix) {
                result = rest.to_string();
                break;
            }
        }
        result
    }

    fn try_parse(s: &str, original: &str) -> SizeUnit {
        // 1. "per kg", "per g", etc.
        if let Some(rest) = s.strip_prefix("per ") {
            let inner = Self::try_parse(rest, original);
            if inner != SizeUnit::Unknown {
                return SizeUnit::PerUnit(Box::new(inner));
            }
        }

        // 2. Volume/weight then pack count: "27l 20pack", "36l 15pack"
        if let Some(result) = Self::parse_unit_then_pack(s) {
            return result;
        }

        // 3. "Npk Xg" format: "3pk 210g", "4pk 12g"
        if let Some(result) = Self::parse_pk_space_unit(s) {
            return result;
        }

        // 4. Multipack formats: "6 x 250ml", "10x22g", "6x 85g", "4 x80g", "8p x 330ml"
        if let Some(result) = Self::parse_multipack(s) {
            return result;
        }

        // 5. Dimension format: "12mm x 15m", "18mmx10m"
        if let Some(result) = Self::parse_dimension(s) {
            return result;
        }

        // 6. Range format: "0.55-0.75kg", "1-2pcs"
        if let Some(result) = Self::parse_range(s) {
            return result;
        }

        // 7. "Nkg pack", "2kg pack" - unit followed by "pack" word
        if let Some(result) = Self::parse_unit_pack(s) {
            return result;
        }

        // 8. Word aliases
        match s {
            "each" | "single" | "ea" | "pk" => return SizeUnit::Unit(1.0),
            _ => {}
        }

        // 9. "N size", "N cup" patterns
        if let Some(result) = Self::parse_count_word(s) {
            return result;
        }

        // 10. "Nea" pattern
        if let Some(value_str) = s.strip_suffix("ea") {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return SizeUnit::Unit(v);
            }
        }

        // 11. "Np" pattern (8p = 8 pieces)
        if let Some(value_str) = s.strip_suffix("p") {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return SizeUnit::Unit(v);
            }
        }

        // 12. Bare units without numbers
        if let Some(unit) = Self::parse_bare_unit(s) {
            return unit;
        }

        // 13. "N+unit" pattern: "8+kg"
        if let Some(result) = Self::parse_plus_unit(s) {
            return result;
        }

        // 14. "Ns" pattern: "100s", "45s"
        if let Some(value_str) = s.strip_suffix('s') {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return SizeUnit::Unit(v);
            }
        }

        // 15. Standard suffixed units: "500g", "1kg", "500ml", etc.
        if let Some(unit) = Self::parse_suffixed_unit(s) {
            return unit;
        }

        // 16. Bare number (assume grams for weight context, or unit)
        if let Ok(v) = s.parse::<f64>() {
            // Small numbers likely units, large numbers likely grams
            if v < 10.0 {
                return SizeUnit::Unit(v);
            } else {
                return Self::kilogram_from_grams(v);
            }
        }

        // Log unrecognized formats
        log_parse_warning("SizeUnit", original, "unrecognized unit format");
        SizeUnit::Unknown
    }

    /// Parse "27l 20pack" → MultiPack(20, Liter(27))
    fn parse_unit_then_pack(s: &str) -> Option<SizeUnit> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() == 2 {
            let unit_part = parts[0];
            let pack_part = parts[1];

            // Check if second part ends with "pack"
            if let Some(count_str) = pack_part.strip_suffix("pack") {
                if let Ok(count) = count_str.parse::<u32>() {
                    let inner = Self::try_parse(unit_part, unit_part);
                    if inner != SizeUnit::Unknown {
                        return Some(SizeUnit::MultiPack { count, unit: Box::new(inner) });
                    }
                }
            }
        }
        None
    }

    /// Parse "3pk 210g" → MultiPack(3, Kilogram(0.21))
    /// Also handles "10 5.2g" → MultiPack(10, Kilogram(0.0052))
    fn parse_pk_space_unit(s: &str) -> Option<SizeUnit> {
        let parts: Vec<&str> = s.split_whitespace().collect();
        if parts.len() == 2 {
            let first = parts[0];
            let second = parts[1];

            // Check if first part is "Npk"
            if let Some(count_str) = first.strip_suffix("pk") {
                if let Ok(count) = count_str.parse::<u32>() {
                    let inner = Self::try_parse(second, second);
                    if inner != SizeUnit::Unknown {
                        return Some(SizeUnit::MultiPack { count, unit: Box::new(inner) });
                    }
                }
            }

            // Check if first part is just a number: "10 5.2g" format
            if let Ok(count) = first.parse::<u32>() {
                let inner = Self::try_parse(second, second);
                if inner != SizeUnit::Unknown {
                    return Some(SizeUnit::MultiPack { count, unit: Box::new(inner) });
                }
            }
        }
        None
    }

    /// Parse multipack formats: "6 x 250ml", "10x22g", "6x 85g", "4 x80g", "8p x 330ml"
    fn parse_multipack(s: &str) -> Option<SizeUnit> {
        // Find 'x' separator
        let x_pos = s.find('x')?;
        let left = s[..x_pos].trim();
        let right = s[x_pos + 1..].trim();

        if left.is_empty() || right.is_empty() {
            return None;
        }

        // Left side: could be "6", "8p", etc.
        let count = if let Some(num_str) = left.strip_suffix("p") {
            num_str.parse::<u32>().ok()?
        } else {
            left.parse::<u32>().ok()?
        };

        // Right side: parse the unit
        let inner = Self::try_parse(right, right);
        if inner != SizeUnit::Unknown {
            return Some(SizeUnit::MultiPack { count, unit: Box::new(inner) });
        }

        None
    }

    /// Parse dimension format: "12mm x 15m", "18mmx10m" - returns first dimension
    fn parse_dimension(s: &str) -> Option<SizeUnit> {
        // Find 'x' and check if both sides have alphabetic chars (units)
        let x_pos = s.find('x')?;
        let left = s[..x_pos].trim();
        let right = s[x_pos + 1..].trim();

        let left_has_unit = left.chars().any(|c| c.is_alphabetic());
        let right_has_unit = right.chars().any(|c| c.is_alphabetic());

        if left_has_unit && right_has_unit {
            let result = Self::try_parse(left, left);
            if result != SizeUnit::Unknown {
                return Some(result);
            }
        }

        None
    }

    /// Parse range format: "0.55-0.75kg", "1-2pcs"
    fn parse_range(s: &str) -> Option<SizeUnit> {
        // Must contain '-' but not start with it
        if !s.contains('-') || s.starts_with('-') {
            return None;
        }

        // Try each unit suffix
        let suffixes: &[(&str, fn(f64) -> SizeUnit)] = &[
            ("kg", SizeUnit::Kilogram),
            ("mg", SizeUnit::Milligram),
            ("gm", Self::kilogram_from_grams),
            ("g", Self::kilogram_from_grams),
            ("ml", Self::liter_from_ml),
            ("l", SizeUnit::Liter),
            ("cm", Self::mm_from_cm),
            ("mm", SizeUnit::Millimeter),
            ("m", SizeUnit::Meter),
            ("pcs", |_| SizeUnit::Unit(1.0)),
            ("pc", |_| SizeUnit::Unit(1.0)),
        ];

        for (suffix, constructor) in suffixes {
            if let Some(range_str) = s.strip_suffix(suffix) {
                if let Some((min_str, max_str)) = range_str.split_once('-') {
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

    /// Parse "1kg pack", "2kg pack" → Kilogram
    fn parse_unit_pack(s: &str) -> Option<SizeUnit> {
        if let Some(unit_str) = s.strip_suffix(" pack") {
            let result = Self::try_parse(unit_str, unit_str);
            if result != SizeUnit::Unknown {
                return Some(result);
            }
        }
        None
    }

    /// Parse "N size", "N cup", "N pellets" → Unit(N)
    fn parse_count_word(s: &str) -> Option<SizeUnit> {
        let count_words = &[" size", " cup", " pellets", " pellet", " count"];
        for word in count_words {
            if let Some(num_str) = s.strip_suffix(word) {
                if let Ok(v) = num_str.trim().parse::<f64>() {
                    return Some(SizeUnit::Unit(v));
                }
            }
        }
        None
    }

    /// Parse "8+kg" → Kilogram(8.0)
    fn parse_plus_unit(s: &str) -> Option<SizeUnit> {
        let suffixes: &[(&str, fn(f64) -> SizeUnit)] = &[
            ("kg", SizeUnit::Kilogram),
            ("g", Self::kilogram_from_grams),
            ("ml", Self::liter_from_ml),
            ("l", SizeUnit::Liter),
        ];

        for (suffix, constructor) in suffixes {
            if let Some(rest) = s.strip_suffix(suffix) {
                if let Some(num_str) = rest.strip_suffix('+') {
                    if let Ok(v) = num_str.parse::<f64>() {
                        return Some(constructor(v));
                    }
                }
            }
        }
        None
    }

    fn parse_bare_unit(s: &str) -> Option<SizeUnit> {
        match s {
            "kg" => Some(SizeUnit::Kilogram(1.0)),
            "g" | "gm" => Some(SizeUnit::Kilogram(0.001)),
            "mg" => Some(SizeUnit::Milligram(1.0)),
            "ml" => Some(SizeUnit::Liter(0.001)),
            "l" => Some(SizeUnit::Liter(1.0)),
            "m" | "mtr" => Some(SizeUnit::Meter(1.0)),
            "cm" => Some(SizeUnit::Millimeter(10.0)),
            "mm" => Some(SizeUnit::Millimeter(1.0)),
            "inch" => Some(SizeUnit::Inch(1.0)),
            _ => None,
        }
    }

    fn parse_suffixed_unit(s: &str) -> Option<SizeUnit> {
        // Count units → Unit
        let count_suffixes = &[
            " sheets", "sheets", " sheet", "sheet",
            " tablets", "tablets", " tabs", "tabs", " caps", "caps",
            " serves", "serves", " serve", "serve",
            " pair", "pair", "pr",
            " pellets", "pellets",
            "pack", "pk", "pcs", "pce", "pc",
        ];
        for suffix in count_suffixes {
            if let Some(value_str) = s.strip_suffix(suffix) {
                if let Ok(v) = value_str.trim().parse::<f64>() {
                    return Some(SizeUnit::Unit(v));
                }
            }
        }

        // Inch
        for suffix in &[" inch", "inch"] {
            if let Some(value_str) = s.strip_suffix(suffix) {
                if let Ok(v) = value_str.trim().parse::<f64>() {
                    return Some(SizeUnit::Inch(v));
                }
            }
        }

        // Weight (check longer suffixes first)
        if let Some(value_str) = s.strip_suffix("kg") {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return Some(SizeUnit::Kilogram(v));
            }
        }
        if let Some(value_str) = s.strip_suffix("mg") {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return Some(SizeUnit::Milligram(v));
            }
        }
        if let Some(value_str) = s.strip_suffix("gm") {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return Some(Self::kilogram_from_grams(v));
            }
        }
        if let Some(value_str) = s.strip_suffix('g') {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return Some(Self::kilogram_from_grams(v));
            }
        }

        // Volume
        if let Some(value_str) = s.strip_suffix("ml") {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return Some(Self::liter_from_ml(v));
            }
        }
        if let Some(value_str) = s.strip_suffix('l') {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return Some(SizeUnit::Liter(v));
            }
        }

        // Length
        if let Some(value_str) = s.strip_suffix("cm") {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return Some(Self::mm_from_cm(v));
            }
        }
        if let Some(value_str) = s.strip_suffix("mm") {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return Some(SizeUnit::Millimeter(v));
            }
        }
        if let Some(value_str) = s.strip_suffix("mtr") {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return Some(SizeUnit::Meter(v));
            }
        }
        if let Some(value_str) = s.strip_suffix('m') {
            if let Ok(v) = value_str.trim().parse::<f64>() {
                return Some(SizeUnit::Meter(v));
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
        assert_eq!(SizeUnit::parse("500g"), SizeUnit::Kilogram(0.5));
        assert_eq!(SizeUnit::parse("1000g"), SizeUnit::Kilogram(1.0));
        assert_eq!(SizeUnit::parse("1kg"), SizeUnit::Kilogram(1.0));
        assert_eq!(SizeUnit::parse("0.5kg"), SizeUnit::Kilogram(0.5));
    }

    #[test]
    fn test_parse_volume_normalizes_to_liter() {
        assert_eq!(SizeUnit::parse("1000ml"), SizeUnit::Liter(1.0));
        assert_eq!(SizeUnit::parse("500ml"), SizeUnit::Liter(0.5));
        assert_eq!(SizeUnit::parse("2l"), SizeUnit::Liter(2.0));
    }

    #[test]
    fn test_parse_unit_followed_by_pack() {
        // "1kg pack" → Kilogram(1.0)
        assert_eq!(SizeUnit::parse("1kg pack"), SizeUnit::Kilogram(1.0));
        assert_eq!(SizeUnit::parse("2kg pack"), SizeUnit::Kilogram(2.0));
    }

    #[test]
    fn test_parse_range_pcs() {
        // "1-2pcs" → Range of Unit
        match SizeUnit::parse("1-2pcs") {
            SizeUnit::Range { min, max, .. } => {
                assert_eq!(min, 1.0);
                assert_eq!(max, 2.0);
            }
            other => panic!("Expected Range, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_pk_space_unit() {
        // "3pk 210g" → MultiPack(3, Kilogram(0.21))
        match SizeUnit::parse("3pk 210g") {
            SizeUnit::MultiPack { count, unit } => {
                assert_eq!(count, 3);
                assert_eq!(*unit, SizeUnit::Kilogram(0.21));
            }
            other => panic!("Expected MultiPack, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_container_suffix_stripped() {
        // "50g pks" → Kilogram(0.05) after stripping "pks"
        assert_eq!(SizeUnit::parse("50g pks"), SizeUnit::Kilogram(0.05));
        assert_eq!(SizeUnit::parse("150g packs"), SizeUnit::Kilogram(0.15));
        assert_eq!(SizeUnit::parse("90g pouches"), SizeUnit::Kilogram(0.09));
        assert_eq!(SizeUnit::parse("90g bars"), SizeUnit::Kilogram(0.09));
    }

    #[test]
    fn test_parse_multipack_variants() {
        // "6x 85g" - space after x
        match SizeUnit::parse("6x 85g") {
            SizeUnit::MultiPack { count, unit } => {
                assert_eq!(count, 6);
                assert_eq!(*unit, SizeUnit::Kilogram(0.085));
            }
            other => panic!("Expected MultiPack, got {:?}", other),
        }

        // "4 x80g" - no space before unit
        match SizeUnit::parse("4 x80g") {
            SizeUnit::MultiPack { count, unit } => {
                assert_eq!(count, 4);
                assert_eq!(*unit, SizeUnit::Kilogram(0.08));
            }
            other => panic!("Expected MultiPack, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_volume_then_pack() {
        // "27L 20pack" → MultiPack(20, Liter(27))
        match SizeUnit::parse("27L 20pack") {
            SizeUnit::MultiPack { count, unit } => {
                assert_eq!(count, 20);
                assert_eq!(*unit, SizeUnit::Liter(27.0));
            }
            other => panic!("Expected MultiPack, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_meter_roll() {
        // "200m roll" → Meter(200)
        assert_eq!(SizeUnit::parse("200m roll"), SizeUnit::Meter(200.0));
    }

    #[test]
    fn test_parse_cup_tray() {
        // "12 cup tray" → Unit(12)
        assert_eq!(SizeUnit::parse("12 cup tray"), SizeUnit::Unit(12.0));
    }

    #[test]
    fn test_parse_dimension_no_space() {
        // "18mmx10m" → Millimeter(18) (first dimension)
        assert_eq!(SizeUnit::parse("18mmx10m"), SizeUnit::Millimeter(18.0));
    }

    #[test]
    fn test_parse_n_size() {
        // "1 size" → Unit(1)
        assert_eq!(SizeUnit::parse("1 size"), SizeUnit::Unit(1.0));
    }

    #[test]
    fn test_parse_bare_number() {
        // "85" - assume grams for larger numbers
        assert_eq!(SizeUnit::parse("85"), SizeUnit::Kilogram(0.085));
    }

    #[test]
    fn test_parse_plus_unit() {
        // "8+kg" → Kilogram(8)
        assert_eq!(SizeUnit::parse("8+kg"), SizeUnit::Kilogram(8.0));
    }

    #[test]
    fn test_parse_np_pattern() {
        // "8p" → Unit(8)
        assert_eq!(SizeUnit::parse("8p"), SizeUnit::Unit(8.0));
    }

    #[test]
    fn test_parse_np_x_unit() {
        // "8p x 330ml" → MultiPack(8, Liter(0.33))
        match SizeUnit::parse("8p x 330ml") {
            SizeUnit::MultiPack { count, unit } => {
                assert_eq!(count, 8);
                assert_eq!(*unit, SizeUnit::Liter(0.33));
            }
            other => panic!("Expected MultiPack, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_pellets() {
        // "96 pellets" → Unit(96)
        assert_eq!(SizeUnit::parse("96 pellets"), SizeUnit::Unit(96.0));
    }

    #[test]
    fn test_parse_double_space() {
        // "10  5.2g" - double space normalized
        match SizeUnit::parse("10  5.2g") {
            SizeUnit::MultiPack { count, .. } => {
                assert_eq!(count, 10);
            }
            other => panic!("Expected MultiPack, got {:?}", other),
        }
    }

    #[test]
    fn test_same_product_normalizes_same() {
        assert_eq!(SizeUnit::parse("500g"), SizeUnit::parse("0.5kg"));
        assert_eq!(SizeUnit::parse("1000ml"), SizeUnit::parse("1l"));
    }
}
