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

impl<T: Entity> Object for EntityMeta<T> {
    fn to_document(&self) -> Result<Document> {
        let Self {
            id,
            created_at,
            updated_at,
        } = self.to_owned();

        let doc = doc! {
            "_id": id.to_object_id(),
            "_created_at": BsonDateTime::from(created_at),
            "_updated_at": BsonDateTime::from(updated_at),
        };
        Ok(doc)
    }

    fn from_document(doc: Document) -> Result<Self> {
        let id = doc
            .get_object_id("_id")
            .context("failed to get _id field")?;
        let created_at = doc
            .get_datetime("_created_at")
            .context("failed to get _created_at field")?;
        let updated_at = doc
            .get_datetime("_updated_at")
            .context("failed to get _updated_at field")?;
        let meta = EntityMeta {
            id: id.into(),
            created_at: created_at.to_chrono(),
            updated_at: updated_at.to_chrono(),
        };
        Ok(meta)
    }
}
