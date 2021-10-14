use super::*;

pub trait EntityConditions {
    fn into_document(self) -> Document;
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EmptyConditions;

impl EntityConditions for EmptyConditions {
    fn into_document(self) -> Document {
        default()
    }
}
