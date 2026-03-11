use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Supermarket {
    #[default]
    Woolworth,
    PakNSave,
    NewWorld,
}

impl Supermarket {
    /// Database ID for this supermarket.
    pub fn id(&self) -> i32 {
        match self {
            Supermarket::NewWorld => 1,
            Supermarket::PakNSave => 2,
            Supermarket::Woolworth => 3,
        }
    }

    /// Display name for this supermarket (used in API responses).
    pub fn name(&self) -> &'static str {
        match self {
            Supermarket::NewWorld => "NewWorld",
            Supermarket::PakNSave => "PakNSave",
            Supermarket::Woolworth => "Woolworth",
        }
    }

    /// Create from database ID.
    pub fn from_id(id: i32) -> Option<Self> {
        match id {
            1 => Some(Supermarket::NewWorld),
            2 => Some(Supermarket::PakNSave),
            3 => Some(Supermarket::Woolworth),
            _ => None,
        }
    }

    /// Create from name string.
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "newworld" | "new world" => Some(Supermarket::NewWorld),
            "paknsave" | "pak'nsave" | "paknnsave" => Some(Supermarket::PakNSave),
            "woolworth" | "woolworths" | "countdown" => Some(Supermarket::Woolworth),
            _ => None,
        }
    }

    /// Returns true if this supermarket has a single virtual store (no per-store pricing).
    pub fn has_single_store(&self) -> bool {
        matches!(self, Supermarket::Woolworth)
    }
}