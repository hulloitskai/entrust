use super::*;

#[derive(Debug)]
pub struct EntityContext<S: EntityServices> {
    pub(super) services: S,
    pub(super) transaction: Option<Arc<Mutex<Transaction>>>,
}

impl<S: EntityServices> Clone for EntityContext<S> {
    fn clone(&self) -> Self {
        let Self {
            services,
            transaction,
        } = self;

        Self {
            services: services.to_owned(),
            transaction: transaction.to_owned(),
        }
    }
}

impl<S: EntityServices> EntityContext<S> {
    pub fn new(services: S) -> Self {
        Self {
            services,
            transaction: None,
        }
    }

    pub fn services(&self) -> &S {
        &self.services
    }
}

impl<S: EntityServices> EntityContext<S> {
    pub async fn transact<F, T, U>(&self, f: F) -> Result<T>
    where
        F: FnOnce(Self) -> U,
        U: Future<Output = Result<T>>,
    {
        self.with_transaction(|ctx, _| f(ctx)).await
    }

    pub(super) async fn with_transaction<F, T, U>(&self, f: F) -> Result<T>
    where
        F: FnOnce(Self, Arc<Mutex<Transaction>>) -> U,
        U: Future<Output = Result<T>>,
    {
        let TransactionState {
            ctx,
            transaction,
            is_root,
        } = self
            .init_transaction()
            .await
            .context("failed to begin transaction")?;

        if is_root {
            let result = f(ctx, transaction.clone()).await;
            if result.is_ok() {
                let mut transaction = transaction.lock().await;
                transaction.commit().await?;
            } else {
                let mut transaction = transaction.lock().await;
                transaction.abort().await?;
            }
            result
        } else {
            f(ctx, transaction).await
        }
    }

    async fn init_transaction(&self) -> Result<TransactionState<S>> {
        let state = match &self.transaction {
            Some(transaction) => TransactionState {
                ctx: self.to_owned(),
                transaction: transaction.to_owned(),
                is_root: false,
            },
            None => {
                let Self { services, .. } = self;
                let transaction = {
                    let client = services.database_client();
                    let transaction = Transaction::new(client).await?;
                    Arc::new(Mutex::new(transaction))
                };
                let ctx = Self {
                    services: services.clone(),
                    transaction: Some(transaction.clone()),
                };
                TransactionState {
                    ctx,
                    transaction,
                    is_root: true,
                }
            }
        };
        Ok(state)
    }
}

impl<S: EntityServices> Deref for EntityContext<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        self.services()
    }
}
