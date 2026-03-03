#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeUnit {
    Kilogram(f64),
    Gram(f64),
    Liter(f64),
    Milliliter(f64),
    Centimeter(f64),
}
