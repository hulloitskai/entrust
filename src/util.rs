use super::*;

use heck::{MixedCase, SnakeCase};

fn transform_bson<F>(bson: Bson, transform: F) -> Bson
where
    F: Copy,
    F: Fn(&str) -> String,
{
    use Bson::*;
    match bson {
        Document(doc) => {
            let doc = transform_document(doc, transform);
            Document(doc)
        }
        Array(items) => {
            let items = items
                .into_iter()
                .map(|item| transform_bson(item, transform))
                .collect::<Vec<_>>();
            Array(items)
        }
        other => other,
    }
}

fn transform_document<F>(mut doc: Document, transform: F) -> Document
where
    F: Copy,
    F: Fn(&str) -> String,
{
    let keys = doc.keys().cloned().collect::<Vec<_>>();
    for key in keys {
        let transformed_key = transform(&key);
        if let Some(val) = doc.remove(key) {
            let transformed_val = transform_bson(val, transform);
            doc.insert(&transformed_key, transformed_val);
        }
    }
    doc
}

pub fn camelize_document(doc: Document) -> Document {
    transform_document(doc, str::to_mixed_case)
}

pub fn snakeify_document(doc: Document) -> Document {
    transform_document(doc, str::to_snake_case)
}
