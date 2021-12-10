use super::*;

pub trait EntityConditions {
    fn to_document(&self) -> Document;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmptyConditions;

impl EntityConditions for EmptyConditions {
    fn to_document(&self) -> Document {
        default()
    }
}
