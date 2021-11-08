use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityMeta<T: Entity> {
    pub id: EntityId<T>,
    pub created_at: DateTime,
    pub updated_at: DateTime,
}

impl<T: Entity> EntityMeta<T> {
    pub fn new() -> Self {
        let created_at = now();
        Self {
            id: default(),
            created_at,
            updated_at: created_at,
        }
    }
}

impl<T: Entity> Default for EntityMeta<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Entity> Clone for EntityMeta<T> {
    fn clone(&self) -> Self {
        let Self {
            id,
            created_at,
            updated_at,
        } = self;

        Self {
            id: *id,
            created_at: *created_at,
            updated_at: *updated_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct EntityMetaDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,

    #[serde(rename = "_createdAt")]
    pub created_at: BsonDateTime,

    #[serde(rename = "_updatedAt")]
    pub updated_at: BsonDateTime,
}

impl<T: Entity> Object for EntityMeta<T> {
    fn to_document(&self) -> Result<Document> {
        let Self {
            id,
            created_at,
            updated_at,
        } = self.to_owned();

        let doc = EntityMetaDocument {
            id: id.to_object_id(),
            created_at: created_at.into(),
            updated_at: updated_at.into(),
        };
        let doc = to_document(&doc)?;
        Ok(doc)
    }

    fn from_document(doc: Document) -> Result<Self> {
        let EntityMetaDocument {
            id,
            created_at,
            updated_at,
        } = from_document(doc)?;

        let meta = EntityMeta {
            id: id.into(),
            created_at: created_at.to_chrono(),
            updated_at: updated_at.to_chrono(),
        };
        Ok(meta)
    }
}
