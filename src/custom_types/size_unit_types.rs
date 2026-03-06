#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeUnit {
    Kilogram(f64),
    Gram(f64),
    Liter(f64),
    Milliliter(f64),
    Centimeter(f64),
    Pack(u32),
    Piece(u32),
}

impl SizeUnit {
    pub fn parse(s: &str) -> Option<SizeUnit> {
        let s = s.to_lowercase();

        if let Some(value_str) = s.strip_suffix("kg") {
            value_str.trim().parse::<f64>().ok().map(SizeUnit::Kilogram)
        } else if let Some(value_str) = s.strip_suffix('g') {
            value_str.trim().parse::<f64>().ok().map(SizeUnit::Gram)
        } else if let Some(value_str) = s.strip_suffix("ml") {
            value_str.trim().parse::<f64>().ok().map(SizeUnit::Milliliter)
        } else if let Some(value_str) = s.strip_suffix('l') {
            value_str.trim().parse::<f64>().ok().map(SizeUnit::Liter)
        } else if let Some(value_str) = s.strip_suffix("cm") {
            value_str.trim().parse::<f64>().ok().map(SizeUnit::Centimeter)
        } else if let Some(value_str) = s.strip_suffix("pack") {
            value_str.trim().parse::<u32>().ok().map(SizeUnit::Pack)
        } else if let Some(value_str) = s.strip_suffix("pk") {
            value_str.trim().parse::<u32>().ok().map(SizeUnit::Pack)
        } else if let Some(value_str) = s.strip_suffix("pcs") {
            value_str.trim().parse::<u32>().ok().map(SizeUnit::Piece)
        } else if let Some(value_str) = s.strip_suffix("pce") {
            value_str.trim().parse::<u32>().ok().map(SizeUnit::Piece)
        } else if let Some(value_str) = s.strip_suffix("pc") {
            value_str.trim().parse::<u32>().ok().map(SizeUnit::Piece)
        } else {
            None
        }
    }
}
