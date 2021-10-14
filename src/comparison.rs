use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Comparison<T> {
    Eq(T),
    Gt(T),
    Gte(T),
    Lt(T),
    Lte(T),
}

impl<T> From<Comparison<T>> for Bson
where
    Bson: From<T>,
{
    fn from(comparison: Comparison<T>) -> Self {
        use Comparison::*;
        match comparison {
            Eq(value) => bson!({ "$eq": value }),
            Gt(value) => bson!({ "$gt": value }),
            Gte(value) => bson!({ "$gte": value }),
            Lt(value) => bson!({ "$lt": value }),
            Lte(value) => bson!({ "$lte": value }),
        }
    }
}

impl<T> From<T> for Comparison<T> {
    fn from(value: T) -> Self {
        Comparison::Eq(value)
    }
}
