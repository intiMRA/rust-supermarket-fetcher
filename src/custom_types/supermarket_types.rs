use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Supermarket {
    #[default]
    Woolworth,
    PakNSave,
    NewWorld,
}
