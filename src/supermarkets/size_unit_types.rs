use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SizeUnit {
    Kilogram(f64),
    Gram(f64),
    Liter(f64),
    Milliliter(f64),
    Centimeter(f64),
    Pack(u32),
    Piece(u32),
    Unknown
}

impl SizeUnit {
    pub fn parse(s: &str) -> SizeUnit {
        let s = s.to_lowercase();

        if let Some(value_str) = s.strip_suffix("kg") {
            value_str.trim().parse::<f64>().map(SizeUnit::Kilogram).unwrap()
        } else if let Some(value_str) = s.strip_suffix('g') {
            value_str.trim().parse::<f64>().map(SizeUnit::Gram).unwrap()
        } else if let Some(value_str) = s.strip_suffix("ml") {
            value_str.trim().parse::<f64>().map(SizeUnit::Milliliter).unwrap()
        } else if let Some(value_str) = s.strip_suffix('l') {
            value_str.trim().parse::<f64>().map(SizeUnit::Liter).unwrap()
        } else if let Some(value_str) = s.strip_suffix("cm") {
            value_str.trim().parse::<f64>().map(SizeUnit::Centimeter).unwrap()
        } else if let Some(value_str) = s.strip_suffix("pack") {
            value_str.trim().parse::<u32>().map(SizeUnit::Pack).unwrap()
        } else if let Some(value_str) = s.strip_suffix("pk") {
            value_str.trim().parse::<u32>().map(SizeUnit::Pack).unwrap()
        } else if let Some(value_str) = s.strip_suffix("pcs") {
            value_str.trim().parse::<u32>().map(SizeUnit::Piece).unwrap()
        } else if let Some(value_str) = s.strip_suffix("pce") {
            value_str.trim().parse::<u32>().map(SizeUnit::Piece).unwrap()
        } else if let Some(value_str) = s.strip_suffix("pc") {
            value_str.trim().parse::<u32>().map(SizeUnit::Piece).unwrap()
        } else {
            SizeUnit::Unknown
        }
    }

    /// Extract the numeric value and unit name as separate parts.
    ///
    /// Useful for storing in database columns.
    ///
    /// # Returns
    /// A tuple of (value, unit_name) where both are strings.
    pub fn to_value_and_unit(&self) -> (f64, &str) {
        match self {
            SizeUnit::Kilogram(v) => (*v, "Kilogram"),
            SizeUnit::Gram(v) => (*v, "Gram"),
            SizeUnit::Liter(v) => (*v, "Liter"),
            SizeUnit::Milliliter(v) => (*v, "Milliliter"),
            SizeUnit::Centimeter(v) => (*v, "Centimeter"),
            SizeUnit::Pack(v) => (*v as f64, "Pack"),
            SizeUnit::Piece(v) => (*v as f64, "Piece"),
            SizeUnit::Unknown => (0.0, "Unknown"),
        }
    }
}
