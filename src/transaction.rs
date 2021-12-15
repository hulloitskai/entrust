use super::*;

#[derive(Derivative)]
#[derivative(Debug)]
pub(super) struct Transaction {
    pub session: DatabaseSession,

    #[derivative(Debug = "ignore")]
    pub commit_finalizers: Vec<BoxFuture<'static, Result<()>>>,

    #[derivative(Debug = "ignore")]
    pub abort_finalizers: Vec<BoxFuture<'static, Result<()>>>,
}

impl Transaction {
    pub async fn new(client: &DatabaseClient) -> Result<Self> {
        let session = {
            let mut session = client
                .start_session(None)
                .await
                .context("failed to start database session")?;
            session.start_transaction(None).await?;
            session
        };
        let transaction = Self {
            session,
            commit_finalizers: default(),
            abort_finalizers: default(),
        };
        Ok(transaction)
    }

    pub async fn commit(&mut self) -> Result<()> {
        let Transaction {
            session,
            commit_finalizers,
            ..
        } = self;
        session.commit_transaction().await?;
        try_join_all(commit_finalizers).await?;
        Ok(())
    }

    pub async fn abort(&mut self) -> Result<()> {
        let Transaction {
            session,
            abort_finalizers,
            ..
        } = self;
        session.abort_transaction().await?;
        try_join_all(abort_finalizers).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct TransactionState<S: EntityServices> {
    pub ctx: EntityContext<S>,
    pub transaction: Arc<Mutex<Transaction>>,
    pub is_root: bool,
}
