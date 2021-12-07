use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityMeta<T: Entity> {
    pub id: EntityId<T>,
    pub created_at: DateTime,
    pub updated_at: DateTime,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime>,
}

impl<T: Entity> EntityMeta<T> {
    pub fn new() -> Self {
        let created_at = now();
        Self {
            id: default(),
            created_at,
            updated_at: created_at,
            deleted_at: default(),
        }
    }

    pub fn is_archived(&self) -> bool {
        self.deleted_at.is_some()
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
            deleted_at,
        } = self;

        Self {
            id: *id,
            created_at: *created_at,
            updated_at: *updated_at,
            deleted_at: *deleted_at,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct EntityMetaDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,

    #[serde(rename = "_createdAt")]
    pub created_at: BsonDateTime,

    #[serde(rename = "_updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<BsonDateTime>,

    #[serde(rename = "_deletedAt", skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<BsonDateTime>,
}

impl<T: Entity> Object for EntityMeta<T> {
    fn to_document(&self) -> Result<Document> {
        let Self {
            id,
            created_at,
            updated_at,
            deleted_at,
        } = self.to_owned();

        let updated_at = if created_at != updated_at {
            Some(updated_at.into())
        } else {
            None
        };

        let doc = EntityMetaDocument {
            id: id.to_object_id(),
            created_at: created_at.into(),
            updated_at,
            deleted_at: deleted_at.map(Into::into),
        };
        let doc = to_document(&doc)?;
        Ok(doc)
    }

    fn from_document(doc: Document) -> Result<Self> {
        let EntityMetaDocument {
            id,
            created_at,
            updated_at,
            deleted_at,
        } = from_document(doc)?;

        let created_at = created_at.to_chrono();
        let updated_at = updated_at
            .map(BsonDateTime::to_chrono)
            .unwrap_or(created_at);

        let meta = EntityMeta {
            id: id.into(),
            created_at,
            updated_at,
            deleted_at: deleted_at.map(BsonDateTime::to_chrono),
        };
        Ok(meta)
    }
}
