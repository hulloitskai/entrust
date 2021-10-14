use super::*;

#[derive(Debug)]
pub(super) struct Transaction {
    pub session: DatabaseSession,
}

impl Transaction {
    pub(super) async fn new(client: &DatabaseClient) -> Result<Self> {
        let mut session = client
            .start_session(None)
            .await
            .context("failed to start database session")?;
        session.start_transaction(None).await?;
        let transaction = Self { session };
        Ok(transaction)
    }

    pub(super) async fn commit(&mut self) -> Result<()> {
        self.session.commit_transaction().await?;
        Ok(())
    }

    pub(super) async fn abort(&mut self) -> Result<()> {
        self.session.abort_transaction().await?;
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct TransactionState<S: EntityServices> {
    pub ctx: EntityContext<S>,
    pub transaction: Arc<Mutex<Transaction>>,
    pub is_root: bool,
}
