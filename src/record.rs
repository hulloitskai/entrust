use super::*;

use mongodb::options::ReplaceOptions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record<T: Entity> {
    #[serde(bound(
        serialize = "T::Id: Serialize",
        deserialize = "T::Id: Deserialize<'de>"
    ))]
    pub meta: EntityMeta<T>,
    pub data: T,
}

impl<T: Entity> Deref for Record<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: Entity> DerefMut for Record<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T: Entity> Record<T> {
    pub fn new(entity: T) -> Self {
        Self {
            meta: default(),
            data: entity,
        }
    }
}

impl<T: Entity> Record<T> {
    pub fn id(&self) -> T::Id {
        self.meta.id
    }

    pub fn created_at(&self) -> DateTime {
        self.meta.created_at
    }

    pub fn updated_at(&self) -> DateTime {
        self.meta.updated_at
    }
}

impl<T: Entity> Record<T> {
    pub async fn save(
        &mut self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<()> {
        self.validate().context("validation failed")?;
        ctx.with_transaction(|ctx, transaction| async move {
            self.before_save(&ctx).await?;
            self.meta.updated_at = now();

            let replacement =
                self.to_document().context("failed to serialize record")?;
            let query = {
                let id: ObjectId = self.id().to_owned().into();
                doc! { "_id": id }
            };
            let collection = T::collection(&ctx);
            let options = ReplaceOptions::builder().upsert(true).build();
            let mut transaction = transaction.lock().await;
            let session = &mut transaction.session;

            trace!(
                collection = collection.name(),
                %query,
                "saving document"
            );
            collection
                .replace_one_with_session(query, replacement, options, session)
                .await?;

            self.after_save(&ctx).await?;
            Ok(())
        })
        .await
    }

    pub async fn delete(
        &mut self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<()> {
        ctx.with_transaction(|ctx, transaction| async move {
            self.before_delete(&ctx).await?;

            let query = {
                let id: ObjectId = self.id().to_owned().into();
                doc! { "_id": id }
            };
            let collection = T::collection(&ctx);
            let mut transaction = transaction.lock().await;
            let session = &mut transaction.session;

            trace!(
                collection = collection.name(),
                %query,
                "deleting document"
            );
            collection
                .delete_one_with_session(query, None, session)
                .await?;

            self.after_delete(&ctx).await?;
            Ok(())
        })
        .await
    }
}

impl<T: Entity> Object for Record<T> {
    fn to_document(&self) -> Result<Document> {
        let Self { meta, data } = self;
        let meta = meta
            .to_document()
            .context("failed to serialize entity meta")?;
        let data = data.to_document().context("failed to serialize entity")?;

        let doc = {
            let mut doc = Document::new();
            doc.extend(data);
            doc.extend(meta);
            doc
        };
        Ok(doc)
    }

    fn from_document(doc: Document) -> Result<Self> {
        let mut meta = Document::new();
        let mut data = Document::new();

        for (key, val) in doc.into_iter() {
            if key.starts_with('_') {
                meta.insert(key, val);
            } else {
                data.insert(key, val);
            }
        }

        let meta = EntityMeta::<T>::from_document(meta)
            .context("failed to deserialize entity meta")?;
        let data =
            T::from_document(data).context("failed to deserialize entity")?;

        let record = Self { meta, data };
        Ok(record)
    }
}
