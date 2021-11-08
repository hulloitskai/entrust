use bson::Document;
use bson::{from_document, to_document};
use serde::{Deserialize, Serialize};

use anyhow::Context as AnyhowContext;
use anyhow::Result;

use entrust::{EmptyConditions, EmptySorting};
use entrust::{Entity, Object, Record, Services};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Person {
    first_name: String,
    last_name: String,
    candies: Vec<Record<Candy>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersonDocument {
    first_name: String,
    last_name: String,
    candies: Vec<Document>,
}

impl Object for Person {
    fn to_document(&self) -> Result<Document> {
        let Person {
            first_name,
            last_name,
            candies,
        } = self;

        let doc = PersonDocument {
            first_name: first_name.to_owned(),
            last_name: last_name.to_owned(),
            candies: candies
                .into_iter()
                .map(Object::to_document)
                .collect::<Result<_>>()
                .context("failed to encode candy to document")?,
        };
        let doc = to_document(&doc)?;
        Ok(doc)
    }

    fn from_document(doc: Document) -> Result<Self> {
        let PersonDocument {
            first_name,
            last_name,
            candies,
        } = from_document(doc)?;

        let person = Person {
            first_name,
            last_name,
            candies: candies
                .into_iter()
                .map(Object::from_document)
                .collect::<Result<_>>()
                .context("failed to decode candy from document")?,
        };
        Ok(person)
    }
}

impl Entity for Person {
    const NAME: &'static str = "Person";

    type Services = Services;
    type Conditions = EmptyConditions;
    type Sorting = EmptySorting;
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
struct Candy {
    wrapper_color: String,
}

impl Entity for Candy {
    const NAME: &'static str = "Candy";

    type Services = Services;
    type Conditions = EmptyConditions;
    type Sorting = EmptySorting;
}

fn main() -> Result<()> {
    let candy_red = Candy {
        wrapper_color: "Red".to_owned(),
    };
    let candy_red = Record::new(candy_red);

    let candy_blue = Candy {
        wrapper_color: "Blue".to_owned(),
    };
    let candy_blue = Record::new(candy_blue);

    let person = Person {
        first_name: "Jon".to_owned(),
        last_name: "Snow".to_owned(),
        candies: vec![candy_red, candy_blue],
    };
    let person = Record::new(person);
    let person_doc = person
        .to_document()
        .context("failed to encode person to document")?;

    println!("person: {:#}", &person_doc);
    Ok(())
}
