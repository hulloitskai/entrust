use super::*;

use mongodb::options::AggregateOptions;
use mongodb::options::CountOptions;
use mongodb::options::FindOneOptions;
use mongodb::options::FindOptions;
use mongodb::options::ReplaceOptions;

use mongodb::error::Result as DatabaseResult;
use mongodb::SessionCursor;

use heck::MixedCase;

#[async_trait]
pub trait Entity
where
    Self: Send + Sync + 'static,
    Self: Clone,
    Self: Object,
{
    const NAME: &'static str;

    type Services: EntityServices;
    type Conditions: EntityConditions;
    type Sorting: EntitySorting;

    fn id(&self) -> EntityId<Self>;

    fn collection_name() -> String {
        Self::NAME.to_mixed_case()
    }

    fn collection(ctx: &EntityContext<Self::Services>) -> Collection<Document> {
        let name = Self::collection_name();
        ctx.database().collection(&name)
    }

    fn get(id: EntityId<Self>) -> FindOneQuery<Self> {
        FindOneQuery::new_untyped(doc! { "_id": id })
    }

    fn get_many(
        ids: impl IntoIterator<Item = EntityId<Self>>,
    ) -> FindQuery<Self> {
        let ids = Bson::from_iter(ids);
        FindQuery::new_untyped(doc! {
            "_id": { "$in": ids }
        })
    }

    fn all() -> FindQuery<Self> {
        Self::find(None)
    }

    fn find(
        conditions: impl Into<Option<Self::Conditions>>,
    ) -> FindQuery<Self> {
        FindQuery::new(conditions)
    }

    fn find_one(
        conditions: impl Into<Option<Self::Conditions>>,
    ) -> FindOneQuery<Self> {
        FindOneQuery::new(conditions)
    }

    fn aggregate<T: Object>(
        pipeline: impl IntoIterator<Item = Document>,
    ) -> AggregateQuery<Self, T> {
        AggregateQuery::new(pipeline)
    }

    fn aggregate_one<T: Object>(
        pipeline: impl IntoIterator<Item = Document>,
    ) -> AggregateOneQuery<Self, T> {
        AggregateOneQuery::new(pipeline)
    }

    async fn count(ctx: &EntityContext<Self::Services>) -> Result<u64> {
        let collection = Self::collection(ctx);
        let count = collection.estimated_document_count(None).await?;
        Ok(count)
    }

    async fn save(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        self.validate().context("validation failed")?;
        let replacement =
            self.to_document().context("failed to serialize record")?;
        ctx.with_transaction(|ctx, transaction| async move {
            let collection = Self::collection(&ctx);
            let id = self.id();
            let conditions = doc! { "_id": &id };
            let options = ReplaceOptions::builder().upsert(true).build();

            let mut transaction = transaction.lock().await;
            let Transaction {
                session,
                commit_finalizers,
                abort_finalizers,
            } = &mut *transaction;
            {
                let entity = self.clone();
                let ctx = ctx.clone();
                let finalizer =
                    async move { entity.after_save_commit(&ctx).await };
                commit_finalizers.push(finalizer.boxed());
            }
            {
                let entity = self.clone();
                let ctx = ctx.clone();
                let finalizer =
                    async move { entity.after_save_abort(&ctx).await };
                abort_finalizers.push(finalizer.boxed());
            }

            self.before_save(&ctx).await?;
            trace!(
                collection = collection.name(),
                %id,
                %conditions,
                "saving document"
            );
            collection
                .replace_one_with_session(
                    conditions,
                    replacement,
                    options,
                    session,
                )
                .await?;
            self.after_save(&ctx).await?;
            Ok(())
        })
        .await
    }

    async fn save_without_callbacks(
        &self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        self.validate().context("validation failed")?;
        let replacement =
            self.to_document().context("failed to serialize record")?;
        ctx.with_transaction(|ctx, transaction| async move {
            let collection = Self::collection(&ctx);
            let id = self.id();
            let conditions = doc! { "_id": &id };
            let options = ReplaceOptions::builder().upsert(true).build();

            let mut transaction = transaction.lock().await;
            let Transaction { session, .. } = &mut *transaction;

            trace!(
                collection = collection.name(),
                %id,
                %conditions,
                "saving document"
            );
            collection
                .replace_one_with_session(
                    conditions,
                    replacement,
                    options,
                    session,
                )
                .await?;
            Ok(())
        })
        .await
    }

    async fn delete(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        ctx.with_transaction(|ctx, transaction| async move {
            let collection = Self::collection(&ctx);
            let id = self.id();
            let conditions = doc! { "_id": &id };

            let mut transaction = transaction.lock().await;
            let Transaction {
                session,
                commit_finalizers,
                abort_finalizers,
            } = &mut *transaction;
            {
                let entity = self.clone();
                let ctx = ctx.clone();
                let finalizer =
                    async move { entity.after_delete_commit(&ctx).await };
                commit_finalizers.push(finalizer.boxed());
            }
            {
                let entity = self.clone();
                let ctx = ctx.clone();
                let finalizer =
                    async move { entity.after_delete_abort(&ctx).await };
                abort_finalizers.push(finalizer.boxed());
            }

            self.before_delete(&ctx).await?;
            trace!(
                collection = collection.name(),
                %id,
                %conditions,
                "deleting document"
            );
            collection
                .delete_one_with_session(doc! { "_id": id }, None, session)
                .await?;
            self.after_delete(&ctx).await?;
            Ok(())
        })
        .await
    }

    async fn delete_without_callbacks(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        ctx.with_transaction(|ctx, transaction| async move {
            let collection = Self::collection(&ctx);
            let id = self.id();
            let conditions = doc! { "_id": &id };

            let mut transaction = transaction.lock().await;
            let Transaction { session, .. } = &mut *transaction;

            trace!(
                collection = collection.name(),
                %id,
                %conditions,
                "deleting document"
            );
            collection
                .delete_one_with_session(doc! { "_id": id }, None, session)
                .await?;
            Ok(())
        })
        .await
    }

    fn validate(&self) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn before_save(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn after_save(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn after_save_commit(
        self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn after_save_abort(
        self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn before_delete(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn after_delete(
        &mut self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn after_delete_commit(
        self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    #[allow(unused_variables)]
    async fn after_delete_abort(
        self,
        ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FindOneQuery<T: Entity>(FindOneQueryInner<T>);

impl<T: Entity> FindOneQuery<T> {
    pub fn new(conditions: impl Into<Option<T::Conditions>>) -> Self {
        let inner = FindOneQueryInner::new(conditions);
        Self(inner)
    }

    pub(super) fn new_untyped(conditions: impl Into<Option<Document>>) -> Self {
        let inner = FindOneQueryInner::new_untyped(conditions);
        Self(inner)
    }

    pub fn optional(self) -> MaybeFindOneQuery<T> {
        let Self(inner) = self;
        MaybeFindOneQuery(inner)
    }

    pub async fn load(self, ctx: &EntityContext<T::Services>) -> Result<T> {
        let Self(inner) = self;
        inner.load(ctx).await?.context("not found")
    }

    pub async fn exists(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<bool> {
        let Self(inner) = self;
        inner.exists(ctx).await
    }
}

#[derive(Debug, Clone)]
pub struct MaybeFindOneQuery<T: Entity>(FindOneQueryInner<T>);

impl<T: Entity> MaybeFindOneQuery<T> {
    pub fn new(conditions: impl Into<Option<T::Conditions>>) -> Self {
        let inner = FindOneQueryInner::new(conditions);
        Self(inner)
    }

    pub fn required(self) -> FindOneQuery<T> {
        let Self(inner) = self;
        FindOneQuery(inner)
    }

    pub async fn load(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<Option<T>> {
        let Self(inner) = self;
        inner.load(ctx).await
    }

    pub async fn exists(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<bool> {
        let Self(inner) = self;
        inner.exists(ctx).await
    }
}

#[derive(Debug, Clone)]
struct FindOneQueryInner<T: Entity> {
    conditions: Option<Document>,
    options: FindOneOptions,
    phantom: PhantomData<T>,
}

impl<T: Entity> FindOneQueryInner<T> {
    pub fn new(conditions: impl Into<Option<T::Conditions>>) -> Self {
        Self::new_untyped({
            let conditions: Option<_> = conditions.into();
            conditions.as_ref().map(EntityConditions::to_document)
        })
    }

    pub fn new_untyped(conditions: impl Into<Option<Document>>) -> Self {
        Self {
            conditions: conditions.into(),
            options: default(),
            phantom: default(),
        }
    }

    pub async fn load(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<Option<T>> {
        let Self {
            conditions,
            options,
            ..
        } = self;
        let collection = T::collection(ctx);

        let doc = if let Some(transaction) = &ctx.transaction {
            let mut transaction = transaction.lock().await;
            let session = &mut transaction.session;
            if let Some(conditions) = &conditions {
                trace!(
                    collection = collection.name(),
                    %conditions,
                    options = %format_find_one_options(&options),
                    session = %session.id(),
                    "finding document"
                );
            } else {
                trace!(
                    collection = collection.name(),
                    options = %format_find_one_options(&options),
                    session = %session.id(),
                    "finding a document"
                );
            }
            collection
                .find_one_with_session(conditions, options, session)
                .await?
        } else {
            if let Some(conditions) = &conditions {
                trace!(
                    collection = collection.name(),
                    %conditions,
                    options = %format_find_one_options(&options),
                    "finding document"
                );
            } else {
                trace!(
                    collection = collection.name(),
                    options = %format_find_one_options(&options),
                    "finding a document"
                );
            }
            collection.find_one(conditions, options).await?
        };

        let doc = match doc {
            Some(doc) => doc,
            None => return Ok(None),
        };
        let object = T::from_document(doc)?;
        Ok(Some(object))
    }

    pub async fn exists(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<bool> {
        let Self { conditions, .. } = self;
        let collection = T::collection(ctx);
        let count = collection.count_documents(conditions, None).await?;
        Ok(count > 0)
    }
}

pub struct FindQuery<T: Entity> {
    conditions: Option<Document>,
    options: FindOptions,
    phantom: PhantomData<T>,
}

fn document_has_operator(doc: &Document, operator: &str) -> bool {
    for (key, value) in doc {
        if key.starts_with('$') {
            if key == operator {
                return true;
            }
            if dbg!(document_value_has_operator(value, operator)) {
                return true;
            }
        }
    }
    false
}

fn document_value_has_operator(value: &Bson, operator: &str) -> bool {
    use Bson::*;
    match value {
        Document(doc) => dbg!(document_has_operator(doc, operator)),
        Array(array) => dbg!(document_array_has_operator(array, operator)),
        _ => false,
    }
}

fn document_array_has_operator(array: &[Bson], operator: &str) -> bool {
    for entry in array {
        if document_value_has_operator(entry, operator) {
            return true;
        }
    }
    false
}

impl<T: Entity> FindQuery<T> {
    pub fn new(conditions: impl Into<Option<T::Conditions>>) -> Self {
        Self::new_untyped({
            let conditions: Option<_> = conditions.into();
            conditions.as_ref().map(EntityConditions::to_document)
        })
    }

    pub(super) fn new_untyped(conditions: impl Into<Option<Document>>) -> Self {
        let conditions: Option<_> = conditions.into();
        let options = {
            let mut options = FindOptions::default();
            if let Some(conditions) = &conditions {
                if document_has_operator(conditions, "$text") {
                    let sort = doc! { "score": { "$meta": "textScore" } };
                    options.sort = Some(sort);
                }
            }
            options
        };
        Self {
            conditions,
            options,
            phantom: default(),
        }
    }

    pub fn and(mut self, conditions: impl Into<Option<T::Conditions>>) -> Self {
        let incoming: Option<Document> = {
            let conditions: Option<_> = conditions.into();
            conditions.as_ref().map(EntityConditions::to_document)
        };
        if let Some(incoming) = incoming {
            let conditions = match self.conditions {
                Some(existing) => {
                    doc! {
                        "$and": [existing, incoming],
                    }
                }
                None => incoming,
            };
            self.conditions = Some(conditions);
        }
        self
    }

    pub fn skip(mut self, n: impl Into<Option<u64>>) -> Self {
        self.options.skip = n.into();
        self
    }

    pub fn take(mut self, n: impl Into<Option<u64>>) -> Self {
        let n: Option<u64> = n.into();
        self.options.limit = n.map(|n| {
            i64::try_from(n).unwrap_or_else(|_| {
                warn!(
                    take = n,
                    "take option has overflowing value; using i64::MAX instead"
                );
                i64::MAX
            })
        });
        self
    }

    pub fn sort(mut self, sorting: impl Into<Option<T::Sorting>>) -> Self {
        let existing = self.options.sort.take();
        let incoming: Option<_> = sorting.into();
        self.options.sort = match incoming {
            Some(incoming) => {
                let incoming = incoming.to_document();
                let combined = match existing {
                    Some(mut existing) => {
                        existing.extend(incoming);
                        existing
                    }
                    None => incoming,
                };
                Some(combined)
            }
            None => existing,
        };
        self
    }

    pub async fn load<'a>(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<impl Stream<Item = Result<T>>> {
        let Self {
            conditions,
            options,
            ..
        } = self;
        let collection = T::collection(ctx);

        let cursor: Box<
            dyn Stream<Item = DatabaseResult<Document>> + Send + Unpin,
        > = if let Some(transaction) = &ctx.transaction {
            let cursor = {
                let mut transaction = transaction.lock().await;
                let session = &mut transaction.session;
                if let Some(conditions) = &conditions {
                    trace!(
                        collection = collection.name(),
                        %conditions,
                        options = %format_find_options(&options),
                        session = %session.id(),
                        "finding documents"
                    );
                } else {
                    trace!(
                        collection = collection.name(),
                        options = %format_find_options(&options),
                        session = %session.id(),
                        "finding all documents"
                    );
                }
                collection
                    .find_with_session(conditions, options, session)
                    .await?
            };
            let cursor = TransactionCursor::new(cursor, transaction.to_owned());
            Box::new(cursor)
        } else {
            if let Some(conditions) = &conditions {
                trace!(
                    collection = collection.name(),
                    %conditions,
                    options = %format_find_options(&options),
                    "finding documents"
                );
            } else {
                trace!(
                    collection = collection.name(),
                    options = %format_find_options(&options),
                    "finding all documents"
                );
            }
            let cursor = collection.find(conditions, options).await?;
            Box::new(cursor)
        };

        let stream = cursor.map(|doc| -> Result<_> {
            let doc = match doc {
                Ok(doc) => doc,
                Err(error) => return Err(error.into()),
            };
            T::from_document(doc)
        });
        Ok(stream)
    }

    pub async fn count(self, ctx: &EntityContext<T::Services>) -> Result<u64> {
        let Self {
            conditions,
            options: find_options,
            ..
        } = self;
        let collection = T::collection(ctx);

        let options = {
            let FindOptions {
                limit,
                skip,
                collation,
                ..
            } = find_options.clone();
            CountOptions::builder()
                .limit(limit.map(|limit| limit as u64))
                .skip(skip)
                .collation(collation)
                .build()
        };

        let count = if let Some(transaction) = &ctx.transaction {
            let mut transaction = transaction.lock().await;
            let session = &mut transaction.session;
            if let Some(conditions) = &conditions {
                trace!(
                    collection = collection.name(),
                    session = %session.id(),
                    %conditions,
                    options = %format_find_options(&find_options),
                    "counting documents"
                );
            } else {
                trace!(
                    collection = collection.name(),
                    session = %session.id(),
                    options = %format_find_options(&find_options),
                    "counting documents"
                );
            }
            collection
                .count_documents_with_session(conditions, options, session)
                .await?
        } else {
            if let Some(conditions) = &conditions {
                trace!(
                    collection = collection.name(),
                    %conditions,
                    options = %format_find_options(&find_options),
                    "counting documents"
                );
            } else {
                trace!(
                    collection = collection.name(),
                    options = %format_find_options(&find_options),
                    "counting documents"
                );
            }
            collection.count_documents(conditions, None).await?
        };

        Ok(count)
    }
}

#[derive(Debug, Clone)]
pub struct AggregateOneQuery<T: Entity, U: Object>(
    AggregateOneQueryInner<T, U>,
);

impl<T: Entity, U: Object> AggregateOneQuery<T, U> {
    pub fn new(pipeline: impl IntoIterator<Item = Document>) -> Self {
        let inner = AggregateOneQueryInner::new(pipeline);
        Self(inner)
    }

    pub fn optional(self) -> MaybeAggregateOneQuery<T, U> {
        let Self(inner) = self;
        MaybeAggregateOneQuery(inner)
    }

    pub async fn load(self, ctx: &EntityContext<T::Services>) -> Result<U> {
        let Self(inner) = self;
        inner.load(ctx).await?.context("not found")
    }
}

#[derive(Debug, Clone)]
pub struct MaybeAggregateOneQuery<T: Entity, U: Object>(
    AggregateOneQueryInner<T, U>,
);

impl<T: Entity, U: Object> MaybeAggregateOneQuery<T, U> {
    pub fn new(pipeline: impl IntoIterator<Item = Document>) -> Self {
        let inner = AggregateOneQueryInner::new(pipeline);
        Self(inner)
    }

    pub fn required(self) -> AggregateOneQuery<T, U> {
        let Self(inner) = self;
        AggregateOneQuery(inner)
    }

    pub async fn load(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<Option<U>> {
        let Self(inner) = self;
        inner.load(ctx).await
    }
}

#[derive(Debug, Clone)]
struct AggregateOneQueryInner<T: Entity, U: Object> {
    pipeline: Vec<Document>,
    phantom_entity: PhantomData<T>,
    phantom_object: PhantomData<U>,
    options: AggregateOptions,
}

impl<T: Entity, U: Object> AggregateOneQueryInner<T, U> {
    pub fn new(pipeline: impl IntoIterator<Item = Document>) -> Self {
        let options = AggregateOptions::default();
        let pipeline = Vec::from_iter(pipeline);
        Self {
            pipeline,
            phantom_entity: default(),
            phantom_object: default(),
            options,
        }
    }

    pub async fn load(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<Option<U>> {
        let Self {
            options, pipeline, ..
        } = self;
        let collection = T::collection(ctx);

        let pipeline = {
            let mut pipeline = pipeline;
            pipeline.push(doc! {
                "$limit": 1
            });
            pipeline
        };

        let mut cursor: Box<
            dyn Stream<Item = DatabaseResult<Document>> + Send + Unpin,
        > = if let Some(transaction) = &ctx.transaction {
            let cursor = {
                let mut transaction = transaction.lock().await;
                let session = &mut transaction.session;
                trace!(
                    collection = collection.name(),
                    session = %session.id(),
                    pipeline = %format_pipeline(&pipeline),
                    "aggregating documents"
                );
                collection
                    .aggregate_with_session(pipeline, options, session)
                    .await?
            };
            let cursor = TransactionCursor::new(cursor, transaction.to_owned());
            Box::new(cursor)
        } else {
            trace!(
                collection = collection.name(),
                pipeline = %format_pipeline(&pipeline),
                "aggregating documents"
            );
            let cursor = collection.aggregate(pipeline, options).await?;
            Box::new(cursor)
        };

        let doc = cursor.next().await;
        let doc = doc.transpose()?;
        let object = doc
            .map(U::from_document)
            .transpose()
            .context("failed to deserialize object")?;
        Ok(object)
    }
}

#[derive(Debug, Clone)]
pub struct AggregateQuery<T: Entity, U: Object> {
    pipeline: Vec<Document>,
    phantom_entity: PhantomData<T>,
    phantom_object: PhantomData<U>,
    options: AggregateOptions,
    skip: Option<u32>,
    take: Option<u32>,
}

impl<T: Entity, U: Object> AggregateQuery<T, U> {
    pub fn new(pipeline: impl IntoIterator<Item = Document>) -> Self {
        let options = AggregateOptions::default();
        let pipeline = Vec::from_iter(pipeline);
        Self {
            pipeline,
            phantom_entity: default(),
            phantom_object: default(),
            options,
            skip: default(),
            take: default(),
        }
    }

    pub fn skip(mut self, n: impl Into<Option<u32>>) -> Self {
        self.skip = n.into();
        self
    }

    pub fn take(mut self, n: impl Into<Option<u32>>) -> Self {
        self.take = n.into();
        self
    }

    pub async fn load<'a>(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<impl Stream<Item = Result<U>>> {
        let Self {
            pipeline,
            options,
            skip,
            take,
            ..
        } = self;
        let collection = T::collection(ctx);

        let pipeline = {
            let mut pipeline = pipeline;
            if let Some(skip) = skip {
                pipeline.push(doc! {
                    "$skip": skip
                });
            }
            if let Some(take) = take {
                pipeline.push(doc! {
                    "$limit": take
                });
            }
            pipeline
        };

        let cursor: Box<
            dyn Stream<Item = DatabaseResult<Document>> + Send + Unpin,
        > = if let Some(transaction) = &ctx.transaction {
            let cursor = {
                let mut transaction = transaction.lock().await;
                let session = &mut transaction.session;
                trace!(
                    collection = collection.name(),
                    session = %session.id(),
                    pipeline = %format_pipeline(&pipeline),
                    options = %format_aggregate_options(&options),
                    "aggregating documents"
                );
                collection
                    .aggregate_with_session(pipeline, options, session)
                    .await?
            };
            let cursor = TransactionCursor::new(cursor, transaction.to_owned());
            Box::new(cursor)
        } else {
            trace!(
                collection = collection.name(),
                pipeline = %format_pipeline(&pipeline),
                options = %format_aggregate_options(&options),
                "aggregating documents"
            );
            let cursor = collection.aggregate(pipeline, options).await?;
            Box::new(cursor)
        };

        let stream = cursor.map(|result| -> Result<U> {
            let doc = result?;
            U::from_document(doc).context("failed to deserialize object")
        });
        Ok(stream)
    }

    pub async fn count<'a>(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<u64> {
        let Self {
            pipeline,
            options,
            skip,
            take,
            ..
        } = self;
        let collection = T::collection(ctx);

        let pipeline = {
            let mut pipeline = pipeline;
            if let Some(skip) = skip {
                pipeline.push(doc! {
                    "$skip": skip
                });
            }
            if let Some(take) = take {
                pipeline.push(doc! {
                    "$limit": take
                });
            }
            pipeline.push(doc! {
                "$count": "_count"
            });
            pipeline
        };

        let result: Document = if let Some(transaction) = &ctx.transaction {
            let mut transaction = transaction.lock().await;
            let session = &mut transaction.session;
            trace!(
                collection = collection.name(),
                session = %session.id(),
                pipeline = %format_pipeline(&pipeline),
                options = %format_aggregate_options(&options),
                "counting aggregated documents"
            );
            let mut cursor = {
                collection
                    .aggregate_with_session(pipeline, options, session)
                    .await?
            };
            cursor.next(session).await.unwrap()?
        } else {
            trace!(
                collection = collection.name(),
                pipeline = %format_pipeline(&pipeline),
                options = %format_aggregate_options(&options),
                "counting aggregated documents"
            );
            let mut cursor = collection.aggregate(pipeline, options).await?;
            cursor.next().await.unwrap()?
        };

        let count = result.get_i64("_count").unwrap();
        let count = u64::try_from(count).unwrap();
        Ok(count)
    }
}

#[pin_project]
#[derive(Debug)]
struct TransactionCursor<T>
where
    T: DeserializeOwned + Unpin,
    T: Send + Sync,
{
    cursor: SessionCursor<T>,
    transaction: Arc<Mutex<Transaction>>,
}

impl<T> TransactionCursor<T>
where
    T: DeserializeOwned + Unpin,
    T: Send + Sync,
{
    fn new(
        cursor: SessionCursor<T>,
        transaction: Arc<Mutex<Transaction>>,
    ) -> Self {
        Self {
            cursor,
            transaction,
        }
    }

    async fn next(self: Pin<&mut Self>) -> Option<DatabaseResult<T>> {
        let projection = self.project();
        let mut transaction = projection.transaction.lock().await;
        let session = &mut transaction.session;
        projection.cursor.next(session).await
    }
}

impl<T> Stream for TransactionCursor<T>
where
    T: DeserializeOwned + Unpin,
    T: Send + Sync,
{
    type Item = DatabaseResult<T>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
    ) -> TaskPoll<Option<Self::Item>> {
        let future = self.next();
        pin_mut!(future);
        future.poll(cx)
    }
}

fn format_find_one_options(options: &FindOneOptions) -> impl Display {
    let options = FindOptions::from(options.to_owned());
    format_find_options(&options)
}

fn format_find_options(options: &FindOptions) -> impl Display {
    to_document(options).unwrap()
}

fn format_aggregate_options(options: &AggregateOptions) -> impl Display {
    to_document(options).unwrap()
}

fn format_pipeline(pipeline: &[Document]) -> impl Display {
    let pipeline = pipeline.iter().cloned().map(Bson::from).collect::<Vec<_>>();
    Bson::Array(pipeline)
}
