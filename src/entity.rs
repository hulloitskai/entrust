use super::*;

use mongodb::options::AggregateOptions;
use mongodb::options::CountOptions;
use mongodb::options::FindOneOptions;
use mongodb::options::FindOptions;

use mongodb::error::Result as DatabaseResult;
use mongodb::SessionCursor;

use heck::MixedCase;

#[async_trait]
pub trait Entity
where
    Self: Object,
    Self: Clone,
    Self: Send + Sync,
{
    const NAME: &'static str;

    type Services: EntityServices;
    type Conditions: EntityConditions;
    type Sorting: EntitySorting;

    fn collection(ctx: &EntityContext<Self::Services>) -> Collection<Document> {
        let name = Self::NAME.to_mixed_case();
        ctx.database().collection(&name)
    }

    fn get(id: EntityId<Self>) -> FindOneQuery<Self> {
        let id = ObjectId::from(id);
        let filter = doc! { "_id": id };
        FindOneQuery::from_filter(filter)
    }

    fn get_many(
        ids: impl IntoIterator<Item = EntityId<Self>>,
    ) -> FindQuery<Self> {
        let ids = {
            let ids = ids.into_iter().map(ObjectId::from);
            Bson::from_iter(ids)
        };
        let filter = doc! { "_id": { "$in": ids } };
        FindQuery::from_filter(filter)
    }

    fn all() -> FindQuery<Self> {
        let filter = doc! { "_deletedAt": { "$exists": false } };
        FindQuery::from_filter(filter)
    }

    fn with_deleted() -> FindQuery<Self> {
        Self::find(None)
    }

    fn only_deleted() -> FindQuery<Self> {
        let filter = doc! { "_deletedAt": { "$exists": true} };
        FindQuery::from_filter(filter)
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

    fn aggregate<U: Object>(
        pipeline: impl IntoIterator<Item = Document>,
    ) -> AggregateQuery<Self, U> {
        AggregateQuery::new(pipeline)
    }

    fn aggregate_one<U: Object>(
        pipeline: impl IntoIterator<Item = Document>,
    ) -> AggregateOneQuery<Self, U> {
        AggregateOneQuery::new(pipeline)
    }

    async fn count(ctx: &EntityContext<Self::Services>) -> Result<u64> {
        let collection = Self::collection(ctx);
        let count = collection.estimated_document_count(None).await?;
        Ok(count)
    }

    fn validate(&self) -> Result<()> {
        Ok(())
    }

    async fn before_save(
        _record: &mut Record<Self>,
        _ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    async fn before_archive(
        _record: &mut Record<Self>,
        _ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    async fn before_restore(
        _record: &mut Record<Self>,
        _ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    async fn before_delete(
        _record: &mut Record<Self>,
        _ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    async fn after_save(
        _record: &mut Record<Self>,
        _ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    async fn after_archive(
        _record: &mut Record<Self>,
        _ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    async fn after_restore(
        _record: &mut Record<Self>,
        _ctx: &EntityContext<Self::Services>,
    ) -> Result<()> {
        Ok(())
    }

    async fn after_delete(
        _record: &mut Record<Self>,
        _ctx: &EntityContext<Self::Services>,
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

    fn from_filter(filter: impl Into<Option<Document>>) -> Self {
        let inner = FindOneQueryInner::from_filter(filter);
        Self(inner)
    }

    pub fn optional(self) -> MaybeFindOneQuery<T> {
        let Self(inner) = self;
        MaybeFindOneQuery(inner)
    }

    pub async fn load(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<Record<T>> {
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
    ) -> Result<Option<Record<T>>> {
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
    filter: Option<Document>,
    options: FindOneOptions,
    phantom: PhantomData<T>,
}

impl<T: Entity> FindOneQueryInner<T> {
    pub fn new(conditions: impl Into<Option<T::Conditions>>) -> Self {
        let filter: Option<Document> = {
            let conditions: Option<_> = conditions.into();
            conditions.map(EntityConditions::into_document)
        };
        Self::from_filter(filter)
    }

    fn from_filter(filter: impl Into<Option<Document>>) -> Self {
        Self {
            filter: filter.into(),
            options: default(),
            phantom: default(),
        }
    }

    pub async fn load(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<Option<Record<T>>> {
        let Self {
            filter, options, ..
        } = self;
        let collection = T::collection(ctx);

        let doc = if let Some(transaction) = &ctx.transaction {
            let mut transaction = transaction.lock().await;
            let session = &mut transaction.session;
            {
                let options = {
                    let options = FindOptions::from(options.clone());
                    to_document(&options).unwrap()
                };
                if let Some(filter) = &filter {
                    trace!(
                        collection = collection.name(),
                        session = %session.id(),
                        %filter,
                        %options,
                        "finding document"
                    );
                } else {
                    trace!(
                        collection = collection.name(),
                        session = %session.id(),
                        %options,
                        "finding a document"
                    );
                }
            }
            collection
                .find_one_with_session(filter, options, session)
                .await?
        } else {
            {
                let options = {
                    let options = FindOptions::from(options.clone());
                    to_document(&options).unwrap()
                };
                if let Some(filter) = &filter {
                    trace!(
                        collection = collection.name(),
                        %filter,
                        %options,
                        "finding document"
                    );
                } else {
                    trace!(
                        collection = collection.name(),
                        %options,
                        "finding a document"
                    );
                }
            }
            collection.find_one(filter, options).await?
        };

        let doc = match doc {
            Some(doc) => doc,
            None => return Ok(None),
        };

        let record = Record::from_document(doc)?;
        Ok(Some(record))
    }

    pub async fn exists(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<bool> {
        let Self { filter, .. } = self;
        let collection = T::collection(ctx);
        let count = collection.count_documents(filter, None).await?;
        Ok(count > 0)
    }
}

pub struct FindQuery<T: Entity> {
    filter: Option<Document>,
    options: FindOptions,
    phantom: PhantomData<T>,
}

fn filter_has_operator(filter: &Document, operator: &str) -> bool {
    for (key, value) in filter {
        if key.starts_with('$') {
            if key == operator {
                return true;
            }
            if dbg!(filter_value_has_operator(value, operator)) {
                return true;
            }
        }
    }
    false
}

fn filter_value_has_operator(value: &Bson, operator: &str) -> bool {
    use Bson::*;
    match value {
        Document(filter) => dbg!(filter_has_operator(filter, operator)),
        Array(array) => dbg!(filter_array_has_operator(array, operator)),
        _ => false,
    }
}

fn filter_array_has_operator(array: &[Bson], operator: &str) -> bool {
    for entry in array {
        if filter_value_has_operator(entry, operator) {
            return true;
        }
    }
    false
}

impl<T: Entity> FindQuery<T> {
    pub fn new(conditions: impl Into<Option<T::Conditions>>) -> Self {
        let filter: Option<Document> = {
            let conditions: Option<_> = conditions.into();
            conditions.map(EntityConditions::into_document)
        };
        Self::from_filter(filter)
    }

    fn from_filter(filter: impl Into<Option<Document>>) -> Self {
        let filter: Option<_> = filter.into();
        let options = {
            let mut options = FindOptions::default();
            if let Some(filter) = &filter {
                if filter_has_operator(filter, "$text") {
                    let sort = doc! { "score": { "$meta": "textScore" } };
                    options.sort = Some(sort);
                }
            }
            options
        };
        Self {
            filter,
            options,
            phantom: default(),
        }
    }

    pub fn and(mut self, conditions: impl Into<Option<T::Conditions>>) -> Self {
        let incoming: Option<Document> = {
            let conditions: Option<_> = conditions.into();
            conditions.map(EntityConditions::into_document)
        };
        if let Some(incoming) = incoming {
            let filter = match self.filter {
                Some(existing) => {
                    doc! {
                        "$and": [existing, incoming],
                    }
                }
                None => incoming,
            };
            self.filter = Some(filter);
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
                let incoming = incoming.into_document();
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
    ) -> Result<impl Stream<Item = Result<Record<T>>>> {
        let Self {
            filter, options, ..
        } = self;
        let collection = T::collection(ctx);

        let cursor: Box<
            dyn Stream<Item = DatabaseResult<Document>> + Send + Unpin,
        > = if let Some(transaction) = &ctx.transaction {
            let cursor = {
                let mut transaction = transaction.lock().await;
                let session = &mut transaction.session;
                if let Some(filter) = &filter {
                    trace!(
                        collection = collection.name(),
                        session = %session.id(),
                        %filter,
                        options = %to_document(&options).unwrap(),
                        "finding documents"
                    );
                } else {
                    trace!(
                        collection = collection.name(),
                        session = %session.id(),
                        options = %to_document(&options).unwrap(),
                        "finding all documents"
                    );
                }
                collection
                    .find_with_session(filter, options, session)
                    .await?
            };
            let cursor = TransactionCursor::new(cursor, transaction.to_owned());
            Box::new(cursor)
        } else {
            if let Some(filter) = &filter {
                trace!(
                    collection = collection.name(),
                    %filter,
                    options = %to_document(&options).unwrap(),
                    "finding documents"
                );
            } else {
                trace!(
                    collection = collection.name(),
                    options = %to_document(&options).unwrap(),
                    "finding all documents"
                );
            }
            let cursor = collection.find(filter, options).await?;
            Box::new(cursor)
        };

        let stream = cursor.map(|doc| -> Result<_> {
            let doc = match doc {
                Ok(doc) => doc,
                Err(error) => return Err(error.into()),
            };
            let record = Record::<T>::from_document(doc)?;
            Ok(record)
        });
        Ok(stream)
    }

    pub async fn count(self, ctx: &EntityContext<T::Services>) -> Result<u64> {
        let Self {
            filter,
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
            {
                let options = to_document(&find_options).unwrap();
                if let Some(filter) = &filter {
                    trace!(
                        collection = collection.name(),
                        session = %session.id(),
                        %filter,
                        %options,
                        "counting documents"
                    );
                } else {
                    trace!(
                        collection = collection.name(),
                        session = %session.id(),
                        %options,
                        "counting documents"
                    );
                }
            }
            collection
                .count_documents_with_session(filter, options, session)
                .await?
        } else {
            {
                let options = to_document(&find_options).unwrap();
                if let Some(filter) = &filter {
                    trace!(
                        collection = collection.name(),
                        %filter,
                        %options,
                        "counting documents"
                    );
                } else {
                    trace!(
                        collection = collection.name(),
                        %options,
                        "counting documents"
                    );
                }
            }
            collection.count_documents(filter, None).await?
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
            options,
            mut pipeline,
            ..
        } = self;

        pipeline.push(doc! {
            "$limit": 1
        });

        let collection = T::collection(ctx);
        let mut cursor: Box<
            dyn Stream<Item = DatabaseResult<Document>> + Send + Unpin,
        > = if let Some(transaction) = &ctx.transaction {
            let cursor = {
                let mut transaction = transaction.lock().await;
                let session = &mut transaction.session;
                trace!(
                    collection = collection.name(),
                    session = %session.id(),
                    pipeline = %bson!(pipeline.clone()),
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
                pipeline = %bson!(pipeline.clone()),
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
            mut pipeline,
            options,
            skip,
            take,
            ..
        } = self;

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

        let collection = T::collection(ctx);
        let cursor: Box<
            dyn Stream<Item = DatabaseResult<Document>> + Send + Unpin,
        > = if let Some(transaction) = &ctx.transaction {
            let cursor = {
                let mut transaction = transaction.lock().await;
                let session = &mut transaction.session;
                trace!(
                    collection = collection.name(),
                    session = %session.id(),
                    pipeline = %bson!(pipeline.clone()),
                    ?options,
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
                pipeline = %bson!(pipeline.clone()),
                ?options,
                "aggregating documents"
            );
            let cursor = collection.aggregate(pipeline, options).await?;
            Box::new(cursor)
        };

        let stream = cursor.map(|doc| -> Result<U> {
            let object = U::from_document(doc?)
                .context("failed to deserialize object")?;
            Ok(object)
        });
        Ok(stream)
    }

    pub async fn count<'a>(
        self,
        ctx: &EntityContext<T::Services>,
    ) -> Result<u64> {
        let Self {
            mut pipeline,
            options,
            skip,
            take,
            ..
        } = self;

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

        let collection = T::collection(ctx);
        let result: Document = if let Some(transaction) = &ctx.transaction {
            let mut transaction = transaction.lock().await;
            let session = &mut transaction.session;
            trace!(
                collection = collection.name(),
                session = %session.id(),
                pipeline = %bson!(pipeline.clone()),
                ?options,
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
                pipeline = %bson!(pipeline.clone()),
                ?options,
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
