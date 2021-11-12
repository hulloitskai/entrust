use super::*;

pub use bson::oid::ObjectId;

#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct EntityId<T: Entity> {
    inner: ObjectId,
    _phantom: PhantomData<T>,
}

impl<T: Entity> Copy for EntityId<T> {}

impl<T: Entity> EntityId<T> {
    pub fn new() -> Self {
        let inner = ObjectId::new();
        Self {
            inner,
            _phantom: default(),
        }
    }
}

impl<T: Entity> EntityId<T> {
    pub fn as_object_id(&self) -> &ObjectId {
        &self.inner
    }

    pub fn to_object_id(&self) -> ObjectId {
        self.inner
    }
}

impl<T: Entity> Default for EntityId<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Entity> From<ObjectId> for EntityId<T> {
    fn from(id: ObjectId) -> Self {
        EntityId {
            inner: id,
            _phantom: default(),
        }
    }
}

impl<T: Entity> From<EntityId<T>> for ObjectId {
    fn from(id: EntityId<T>) -> Self {
        id.inner
    }
}

impl<T: Entity> From<EntityId<T>> for Bson {
    fn from(id: EntityId<T>) -> Self {
        id.to_object_id().into()
    }
}

impl<T: Entity> Debug for EntityId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let s = format!("{}:{}", T::NAME, &self.inner);
        f.write_str(&s)
    }
}

impl<T: Entity> Display for EntityId<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let s = encode_base64(format!("{:?}", self));
        f.write_str(&s)
    }
}

impl<T: Entity> Serialize for EntityId<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = self.to_string();
        s.serialize(serializer)
    }
}

impl<'de, T: Entity> Deserialize<'de> for EntityId<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(|error| {
            let message = format!("{:?}", error);
            D::Error::custom(message)
        })
    }
}

impl<T: Entity> FromStr for EntityId<T> {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = {
            let data = decode_base64(s).context("failed to decode base64")?;
            String::from_utf8_lossy(&data[..]).into_owned()
        };
        let segments = s.split(':').collect::<Vec<_>>();

        let (id, entity_name) = match segments[..] {
            [entity_name, id] => {
                let id: ObjectId =
                    id.parse().context("failed to parse ObjectId")?;
                (id, entity_name.to_owned())
            }
            _ => bail!("bad format"),
        };
        if entity_name != T::NAME {
            bail!(
                "incorrect entity name: expected {}, got {}",
                T::NAME,
                entity_name
            );
        }

        let id = Self::from(id);
        Ok(id)
    }
}
