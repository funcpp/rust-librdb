pub type Result<T> = std::result::Result<T, RdbError>;

#[derive(Debug, thiserror::Error)]
pub enum RdbError {
    #[error("parser error (code {code}): {message}")]
    Parser { code: u32, message: String },

    #[error("{0}")]
    Handler(Box<dyn std::error::Error + Send + Sync>),
}

impl RdbError {
    pub fn handler(e: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Handler(Box::new(e))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive] // RDB gains data types across versions (e.g. Array in v14); keep this open.
pub enum DataType {
    String,
    List,
    Set,
    Zset,
    Hash,
    Module,
    Stream,
    Function,
    /// Sparse array (RDB v14+, `RDB_TYPE_ARRAY`).
    Array,
}

impl DataType {
    pub(crate) fn from_raw(value: i32) -> Result<Self> {
        #![allow(non_upper_case_globals)]
        use librdb_sys::{
            RdbDataType_RDB_DATA_TYPE_ARRAY, RdbDataType_RDB_DATA_TYPE_FUNCTION,
            RdbDataType_RDB_DATA_TYPE_HASH, RdbDataType_RDB_DATA_TYPE_LIST,
            RdbDataType_RDB_DATA_TYPE_MODULE, RdbDataType_RDB_DATA_TYPE_SET,
            RdbDataType_RDB_DATA_TYPE_STREAM, RdbDataType_RDB_DATA_TYPE_STRING,
            RdbDataType_RDB_DATA_TYPE_ZSET,
        };
        #[allow(clippy::cast_sign_loss)]
        match value as u32 {
            RdbDataType_RDB_DATA_TYPE_STRING => Ok(Self::String),
            RdbDataType_RDB_DATA_TYPE_LIST => Ok(Self::List),
            RdbDataType_RDB_DATA_TYPE_SET => Ok(Self::Set),
            RdbDataType_RDB_DATA_TYPE_ZSET => Ok(Self::Zset),
            RdbDataType_RDB_DATA_TYPE_HASH => Ok(Self::Hash),
            RdbDataType_RDB_DATA_TYPE_MODULE => Ok(Self::Module),
            RdbDataType_RDB_DATA_TYPE_STREAM => Ok(Self::Stream),
            RdbDataType_RDB_DATA_TYPE_FUNCTION => Ok(Self::Function),
            RdbDataType_RDB_DATA_TYPE_ARRAY => Ok(Self::Array),
            _ => Err(RdbError::Parser {
                code: 0,
                message: format!("unknown data type: {value}"),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInfo {
    /// Unix timestamp in milliseconds, or `None` if no expiry is set.
    pub expire_time_ms: Option<i64>,
    /// LRU idle seconds, or `None` if not available.
    pub lru_idle: Option<i64>,
    /// LFU frequency counter, or `None` if not available.
    pub lfu_freq: Option<i32>,
    pub data_type: DataType,
}

impl KeyInfo {
    pub(crate) fn from_raw(raw: &librdb_sys::RdbKeyInfo) -> Result<Self> {
        Ok(Self {
            expire_time_ms: if raw.expiretime == -1 {
                None
            } else {
                Some(raw.expiretime)
            },
            lru_idle: if raw.lruIdle == -1 {
                None
            } else {
                Some(raw.lruIdle)
            },
            lfu_freq: if raw.lfuFreq == -1 {
                None
            } else {
                Some(raw.lfuFreq)
            },
            data_type: DataType::from_raw(raw.dataType)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotInfo {
    pub slot_id: u64,
    pub slot_size: u64,
    pub expires_slot_size: u64,
}

impl SlotInfo {
    pub(crate) const fn from_raw(raw: &librdb_sys::RdbSlotInfo) -> Self {
        Self {
            slot_id: raw.slot_id,
            slot_size: raw.slot_size,
            expires_slot_size: raw.expires_slot_size,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamId {
    pub ms: u64,
    pub seq: u64,
}

impl StreamId {
    pub(crate) const fn from_raw(raw: &librdb_sys::RdbStreamID) -> Self {
        Self {
            ms: raw.ms,
            seq: raw.seq,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StreamMeta {
    pub length: u64,
    pub entries_added: u64,
    pub first_id: StreamId,
    pub last_id: StreamId,
    pub max_del_entry_id: StreamId,
}

impl StreamMeta {
    pub(crate) const fn from_raw(raw: &librdb_sys::RdbStreamMeta) -> Self {
        Self {
            length: raw.length,
            entries_added: raw.entriesAdded,
            first_id: StreamId::from_raw(&raw.firstID),
            last_id: StreamId::from_raw(&raw.lastID),
            max_del_entry_id: StreamId::from_raw(&raw.maxDelEntryID),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamGroupMeta {
    pub last_id: StreamId,
    pub entries_read: i64,
}

impl StreamGroupMeta {
    pub(crate) const fn from_raw(raw: &librdb_sys::RdbStreamGroupMeta) -> Self {
        Self {
            last_id: StreamId::from_raw(&raw.lastId),
            entries_read: raw.entriesRead,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamConsumerMeta {
    pub active_time: i64,
    pub seen_time: i64,
}

impl StreamConsumerMeta {
    pub(crate) const fn from_raw(raw: &librdb_sys::RdbStreamConsumerMeta) -> Self {
        Self {
            active_time: raw.activeTime,
            seen_time: raw.seenTime,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamPendingEntry {
    pub id: StreamId,
    pub delivery_time: u64,
    pub delivery_count: u64,
}

impl StreamPendingEntry {
    pub(crate) const fn from_raw(raw: &librdb_sys::RdbStreamPendingEntry) -> Self {
        Self {
            id: StreamId::from_raw(&raw.id),
            delivery_time: raw.deliveryTime,
            delivery_count: raw.deliveryCount,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StreamIdmpMeta {
    pub duration: u64,
    pub max_entries: u64,
    pub num_producers: u64,
}

impl StreamIdmpMeta {
    pub(crate) const fn from_raw(raw: &librdb_sys::RdbStreamIdmpMeta) -> Self {
        Self {
            duration: raw.duration,
            max_entries: raw.maxEntries,
            num_producers: raw.numProducers,
        }
    }
}
