// Copyright (c) 2020, KTH Royal Institute of Technology.
// SPDX-License-Identifier: AGPL-3.0-only

#[allow(dead_code)]
pub mod appender;
pub mod hash_table;
pub mod timer;
pub mod value;
pub mod window;

use crate::error::ArconResult;
use crate::stream::operator::window::WindowContext;
use crate::ArconType;
use crate::{data::arrow::ToArrow, manager::snapshot::Snapshot, table::ImmutableTable};
use arcon_state::{
    data::{Key, Value},
    error::Result,
    Backend,
};
use std::{borrow::Cow, sync::Arc};

pub trait IndexValue: Value + ToArrow {}
impl<T> IndexValue for T where T: Value + ToArrow {}

pub use self::{
    appender::eager::EagerAppender,
    hash_table::{eager::EagerHashTable, HashTable},
    timer::{Timer, TimerEvent},
    value::{EagerValue, LazyValue, LocalValue},
    window::appender::AppenderWindow,
    window::arrow::ArrowWindow,
    window::incremental::IncrementalWindow,
};

/// Common Index Operations
///
/// All indexes must implement the IndexOps trait
pub trait IndexOps {
    /// This method ensures all non-persisted data gets pushed to a Backend
    fn persist(&mut self) -> ArconResult<()>;
    /// Set the current active key for the index
    fn set_key(&mut self, key: u64);

    /// Create a [ImmutableTable] from the data in the Index
    fn table(&mut self) -> ArconResult<Option<ImmutableTable>>;
}

/// Active Arcon State
pub trait ArconState: Send + 'static {
    const STATE_ID: &'static str;

    /// Restores an ArconState from a [Snapshot]
    fn restore<B: Backend>(snapshot: Snapshot, f: Arc<dyn Fn(Arc<B>) -> Self>) -> ArconResult<Self>
    where
        Self: Sized,
    {
        let snapshot_dir = std::path::Path::new(&snapshot.snapshot_path);
        let backend = B::restore(snapshot_dir, snapshot_dir, String::from(Self::STATE_ID))?;
        Ok(f(Arc::new(backend)))
    }

    fn persist(&mut self) -> ArconResult<()>;
    fn set_key(&mut self, key: u64);

    /// Returns a Vec of registered tables
    fn tables(&mut self) -> Vec<ImmutableTable>;

    fn table_ids() -> Vec<String>;

    fn get_table(&mut self, id: &str) -> ArconResult<Option<ImmutableTable>>;

    fn has_tables() -> bool;
}

/// Identifier for empty ArconState
pub const EMPTY_STATE_ID: &str = "!";

/// Struct used to signal an empty ArconState implementation
pub struct EmptyState;

impl ArconState for EmptyState {
    const STATE_ID: &'static str = EMPTY_STATE_ID;

    fn persist(&mut self) -> ArconResult<()> {
        Ok(())
    }
    fn set_key(&mut self, _: u64) {}
    fn tables(&mut self) -> Vec<ImmutableTable> {
        Vec::new()
    }
    fn table_ids() -> Vec<String> {
        Vec::new()
    }
    fn get_table(&mut self, _: &str) -> ArconResult<Option<ImmutableTable>> {
        Ok(None)
    }
    fn has_tables() -> bool {
        false
    }
}

impl IndexOps for EmptyState {
    fn persist(&mut self) -> ArconResult<()> {
        Ok(())
    }
    fn set_key(&mut self, _: u64) {
        // ignore
    }
    fn table(&mut self) -> ArconResult<Option<ImmutableTable>> {
        Ok(None)
    }
}

/// Index for Maintaining an Appender per Key
///
/// Keys are set by the Arcon runtime.
pub trait AppenderIndex<V>: Send + Sized + IndexOps + 'static
where
    V: Value,
{
    /// Add data to an Appender
    fn append(&mut self, value: V) -> Result<()>;
    /// Consumes the Appender
    ///
    /// Safety: Note that this call loads the data eagerly and may lead to problems if there is a
    /// lack of system memory.
    fn consume(&mut self) -> Result<Vec<V>>;
    /// Returns the length of the Appender
    fn len(&self) -> usize;
    /// Method to check whether an Appender is empty
    fn is_empty(&self) -> bool;
}

/// Index for Maintaining a single value per Key
///
/// Keys are set by the Arcon runtime.
pub trait ValueIndex<V>: Send + Sized + IndexOps + 'static
where
    V: Value,
{
    /// Blind update of the current value
    fn put(&mut self, value: V) -> Result<()>;
    /// Fetch the current value.
    ///
    /// The returned value is wrapped in a [Cow] in order to
    /// support both owned and referenced values depending on
    /// whether the index is Eager or Lazy.
    fn get(&self) -> Result<Option<Cow<V>>>;
    /// Take the value out
    ///
    /// Returns `Some(V)` if the value exists or `None` if it does not.
    fn take(&mut self) -> Result<Option<V>>;
    /// Clear value if it exists
    fn clear(&mut self) -> Result<()>;
    /// Read-Modify-Write operation
    ///
    /// If the value does not exist, V::Default will be inserted.
    fn rmw<F>(&mut self, f: F) -> Result<()>
    where
        F: FnMut(&mut V) + Sized;
}

/// Index for Maintaining a Map per Key
///
/// Keys are set by the Arcon runtime.
pub trait MapIndex<K, V>: Send + Sized + IndexOps + 'static
where
    K: Key,
    V: Value,
{
    /// Blind insert
    fn put(&mut self, key: &K, value: V) -> Result<()>;
    /// Fetch Value by Key
    fn get(&self, key: &K) -> Result<Option<V>>;
    /// Attempt to take the value out of the Map
    fn take(&mut self, key: &K) -> Result<Option<V>>;
    /// Clear value by key
    fn clear(&mut self, key: &K) -> Result<()>;
    /// Length of the current Map
    fn len(&self) -> usize;
    /// Checks whether the Map is empty
    fn is_empty(&self) -> bool;
    /// Read-Modify-Write operation
    fn rmw<F>(&mut self, key: &K, value: V)
    where
        F: FnMut(&mut V) + Sized;
}

/// Index for Streaming Windows
///
/// Contains all the methods a Window must implement
pub trait WindowIndex: Send + Sized + IndexOps + 'static {
    type IN: ArconType;
    type OUT: ArconType;

    /// The `on_element` function is called per received window element
    fn on_element(&mut self, element: Self::IN, ctx: WindowContext) -> ArconResult<()>;
    /// The `result` function is called at the end of a window's lifetime
    fn result(&mut self, ctx: WindowContext) -> ArconResult<Self::OUT>;
    /// Clears the window state for the passed context
    fn clear(&mut self, ctx: WindowContext) -> ArconResult<()>;
}
