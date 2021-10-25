pub use macros::Object;

mod object;
pub use object::*;

mod transaction;
use transaction::*;

mod database;
pub use database::*;

mod services;
pub use services::*;

mod context;
pub use context::*;

mod id;
pub use id::*;

mod meta;
pub use meta::*;

mod entity;
pub use entity::*;

mod record;
pub use record::*;

mod comparison;
pub use comparison::*;

mod conditions;
pub use conditions::*;

mod sorting;
pub use sorting::*;

use std::convert::TryFrom;
use std::fmt::Result as FmtResult;
use std::fmt::{Debug, Display, Formatter};
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::Context as TaskContext;
use std::task::Poll as TaskPoll;

use anyhow::bail;
use anyhow::Context as AnyhowContext;
use anyhow::{Error, Result};

use futures::{Future, Stream};
use futures_util::pin_mut;
use futures_util::StreamExt;

use base64::decode_config as decode_base64_config;
use base64::encode_config as encode_base64_config;
use base64::DecodeError as Base64DecodeError;
use base64::URL_SAFE as BASE64_CONFIG;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use bson::DateTime as BsonDateTime;
use bson::{bson, doc, to_document};
use bson::{Bson, Document};

use async_trait::async_trait;
use pin_project::pin_project;
use tokio::sync::Mutex;
use tracing::trace;
use typed_builder::TypedBuilder as Builder;

use chrono::DateTime as ChronoDateTime;
use chrono::Utc;

type DateTime = ChronoDateTime<Utc>;

fn default<T: Default>() -> T {
    T::default()
}

fn now() -> DateTime {
    Utc::now()
}