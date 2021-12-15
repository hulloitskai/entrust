use bson::Document;
use bson::{doc, from_document, to_document};

use mongodb::options::ClientOptions as MongoClientOptions;
use mongodb::Client as MongoClient;

use entrust::{EmptyConditions, EmptySorting, Services};
use entrust::{Entity, EntityContext, EntityId};
use entrust::{Object, ObjectId};

use anyhow::Context as AnyhowContext;
use anyhow::Result;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::main as tokio;
use typed_builder::TypedBuilder as Builder;

type UserId = EntityId<User>;

#[derive(Debug, Clone, Serialize, Deserialize, Builder)]
struct User {
    #[builder(default, setter(skip))]
    pub id: UserId,

    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserDocument {
    #[serde(rename = "_id")]
    pub id: ObjectId,

    pub name: String,
}

impl From<User> for UserDocument {
    fn from(user: User) -> Self {
        let User { id, name } = user;
        UserDocument {
            id: id.into(),
            name,
        }
    }
}

impl From<UserDocument> for User {
    fn from(doc: UserDocument) -> Self {
        let UserDocument { id: _id, name } = doc;
        User {
            id: _id.into(),
            name,
        }
    }
}

impl Object for User {
    fn to_document(&self) -> Result<bson::Document> {
        let doc = UserDocument::from(self.clone());
        let doc = to_document(&doc)?;
        Ok(doc)
    }

    fn from_document(doc: Document) -> Result<Self> {
        let doc = from_document::<UserDocument>(doc)?;
        let user = User::from(doc);
        Ok(user)
    }
}

#[async_trait]
impl Entity for User {
    const NAME: &'static str = "User";

    type Services = Services;
    type Conditions = EmptyConditions;
    type Sorting = EmptySorting;

    fn id(&self) -> EntityId<Self> {
        self.id
    }

    async fn before_save(
        &mut self,
        _: &EntityContext<Self::Services>,
    ) -> Result<()> {
        println!("before save");
        Ok(())
    }

    async fn after_save(
        &mut self,
        _: &EntityContext<Self::Services>,
    ) -> Result<()> {
        println!("after save");
        Ok(())
    }

    async fn after_save_commit(
        self,
        _: &EntityContext<Self::Services>,
    ) -> Result<()> {
        println!("after save commit");
        Ok(())
    }

    async fn after_save_abort(
        self,
        _: &EntityContext<Self::Services>,
    ) -> Result<()> {
        println!("after save abort");
        Ok(())
    }
}

#[tokio]
async fn main() -> Result<()> {
    run().await
}

async fn run() -> Result<()> {
    let database_client = MongoClient::with_options({
        let uri = "mongodb://localhost:27017";
        let mut options = MongoClientOptions::parse(uri)
            .await
            .context("failed to parse MongoDB connection string")?;
        options.retry_writes = Some(true);
        options
    })
    .context("failed to build MongoDB client")?;

    let database = {
        let database = database_client.database("entrust");
        database
            .run_command(doc! { "ping": 1 }, None)
            .await
            .context("failed to connect to MongoDB")?;
        database
    };

    let services = Services::builder()
        .database_client(database_client)
        .database(database)
        .build();
    let ctx = EntityContext::new(services);

    let mut user = User::builder().name("George".to_owned()).build();
    println!("saving user");
    user.save(&ctx).await.context("failed to save user")?;
    println!("done saving user");

    Ok(())
}
