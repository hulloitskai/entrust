use super::*;

pub trait EntitySorting {
    fn to_document(&self) -> Document;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmptySorting;

impl EntitySorting for EmptySorting {
    fn to_document(&self) -> Document {
        default()
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortingDirection {
    Asc,
    Desc,
}

impl Default for SortingDirection {
    fn default() -> Self {
        Self::Asc
    }
}

impl From<SortingDirection> for Bson {
    fn from(direction: SortingDirection) -> Self {
        use SortingDirection::*;
        let direction = match direction {
            Asc => 1,
            Desc => -1,
        };
        direction.into()
    }
}
