use crate::types::{
    KeyInfo, Result, SlotInfo, StreamConsumerMeta, StreamGroupMeta, StreamId, StreamIdmpMeta,
    StreamMeta, StreamPendingEntry,
};

/// Level2 (`RDB_LEVEL_DATA`) callback handler for Redis RDB parsing.
///
/// Each method has a default no-op implementation returning `Ok(())`.
/// Implement only the callbacks you need.
///
/// The parsing flow for a key is:
///   `handle_new_key` → one or more value/element callbacks → `handle_end_key`
///
/// `&[u8]` parameters borrow from librdb's internal bulk-pool and are only
/// valid for the duration of the callback. Clone explicitly if you need to
/// retain the data.
#[allow(clippy::missing_errors_doc)]
pub trait RdbHandlers {
    fn handle_start_rdb(&mut self, _rdb_version: i32) -> Result<()> {
        Ok(())
    }

    fn handle_end_rdb(&mut self) -> Result<()> {
        Ok(())
    }

    fn handle_new_db(&mut self, _db_number: i32) -> Result<()> {
        Ok(())
    }

    fn handle_db_size(&mut self, _db_size: u64, _expires_size: u64) -> Result<()> {
        Ok(())
    }

    fn handle_slot_info(&mut self, _info: &SlotInfo) -> Result<()> {
        Ok(())
    }

    fn handle_aux_field(&mut self, _key: &[u8], _value: &[u8]) -> Result<()> {
        Ok(())
    }

    fn handle_new_key(&mut self, _key: &[u8], _info: &KeyInfo) -> Result<()> {
        Ok(())
    }

    fn handle_end_key(&mut self) -> Result<()> {
        Ok(())
    }

    fn handle_string_value(&mut self, _value: &[u8]) -> Result<()> {
        Ok(())
    }

    fn handle_list_item(&mut self, _item: &[u8]) -> Result<()> {
        Ok(())
    }

    /// `expire_at`: Unix timestamp in ms for field-level TTL (Redis 7.4+), -1 if not set.
    fn handle_hash_field(&mut self, _field: &[u8], _value: &[u8], _expire_at: i64) -> Result<()> {
        Ok(())
    }

    fn handle_set_member(&mut self, _member: &[u8]) -> Result<()> {
        Ok(())
    }

    fn handle_zset_member(&mut self, _member: &[u8], _score: f64) -> Result<()> {
        Ok(())
    }

    fn handle_function(&mut self, _func: &[u8]) -> Result<()> {
        Ok(())
    }

    fn handle_module(&mut self, _module_name: &[u8], _serialized_size: usize) -> Result<()> {
        Ok(())
    }

    fn handle_stream_metadata(&mut self, _meta: &StreamMeta) -> Result<()> {
        Ok(())
    }

    fn handle_stream_item(
        &mut self,
        _id: &StreamId,
        _field: &[u8],
        _value: &[u8],
        _items_left: i64,
    ) -> Result<()> {
        Ok(())
    }

    fn handle_stream_new_cgroup(
        &mut self,
        _group_name: &[u8],
        _meta: &StreamGroupMeta,
    ) -> Result<()> {
        Ok(())
    }

    fn handle_stream_cgroup_pending_entry(&mut self, _entry: &StreamPendingEntry) -> Result<()> {
        Ok(())
    }

    fn handle_stream_new_consumer(
        &mut self,
        _consumer_name: &[u8],
        _meta: &StreamConsumerMeta,
    ) -> Result<()> {
        Ok(())
    }

    fn handle_stream_consumer_pending_entry(&mut self, _id: &StreamId) -> Result<()> {
        Ok(())
    }

    /// A NACK-zone (not-yet-acknowledged) entry within a consumer group
    /// (`RDB_TYPE_STREAM_LISTPACKS_5`, RDB v14+).
    fn handle_stream_nack_zone_entry(&mut self, _id: &StreamId, _items_left: i64) -> Result<()> {
        Ok(())
    }

    fn handle_stream_idmp_meta(&mut self, _meta: &StreamIdmpMeta) -> Result<()> {
        Ok(())
    }

    fn handle_stream_idmp_producer(&mut self, _pid: &[u8], _num_entries: u64) -> Result<()> {
        Ok(())
    }

    fn handle_stream_idmp_entry(&mut self, _iid: &[u8], _stream_id: &StreamId) -> Result<()> {
        Ok(())
    }

    /// Metadata for a sparse array (`RDB_TYPE_ARRAY`, RDB v14+), called once
    /// before its elements.
    ///
    /// `insert_idx` is the persisted insert cursor, or `None` if the array was
    /// saved without one (librdb's `RDB_ARRAY_INSERT_IDX_NONE` sentinel).
    fn handle_array_metadata(&mut self, _count: u64, _insert_idx: Option<u64>) -> Result<()> {
        Ok(())
    }

    /// A single sparse-array element, called `count` times in ascending `idx` order.
    fn handle_array_element(&mut self, _idx: u64, _value: &[u8]) -> Result<()> {
        Ok(())
    }
}
