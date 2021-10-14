use super::*;

pub trait EntityServices: Send + Sync {
    fn database(&self) -> &Database;
    fn database_client(&self) -> &DatabaseClient;
}

#[derive(Debug, Clone, Builder)]
pub struct Services {
    database: Database,
    database_client: DatabaseClient,
}

impl EntityServices for Services {
    fn database(&self) -> &Database {
        &self.database
    }

    fn database_client(&self) -> &DatabaseClient {
        &self.database_client
    }
}
