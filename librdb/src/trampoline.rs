//! C-to-Rust trampoline functions — the sole location of `unsafe` in this crate.
//!
//! All work in each trampoline (type conversion, handler call) is wrapped in
//! `catch_unwind` to prevent panics from unwinding through C stack frames.
#![allow(unsafe_code)]

use std::{os::raw::c_void, panic::AssertUnwindSafe};

use librdb_sys::{
    self, RdbBulk, RdbKeyInfo, RdbParser, RdbRes, RdbRes_RDB_ERR_CANCEL_PARSING, RdbRes_RDB_OK,
    RdbSlotInfo, RdbStreamConsumerMeta, RdbStreamGroupMeta, RdbStreamID, RdbStreamIdmpEntry,
    RdbStreamIdmpMeta, RdbStreamIdmpProducer, RdbStreamMeta, RdbStreamPendingEntry,
};

use crate::{
    handlers::RdbHandlers,
    types::{
        self, KeyInfo, RdbError, SlotInfo, StreamConsumerMeta as RsStreamConsumerMeta,
        StreamGroupMeta as RsStreamGroupMeta, StreamId, StreamIdmpMeta as RsStreamIdmpMeta,
        StreamMeta as RsStreamMeta, StreamPendingEntry as RsStreamPendingEntry,
    },
};

pub struct HandlerState<H> {
    pub handler: H,
    pub last_error: Option<RdbError>,
}

/// # Safety
/// `parser` and `bulk` must be valid pointers within an active librdb callback.
unsafe fn bulk_to_slice<'a>(parser: *mut RdbParser, bulk: RdbBulk) -> &'a [u8] {
    let len = unsafe { librdb_sys::RDB_bulkLen(parser, bulk) };
    unsafe { std::slice::from_raw_parts(bulk.cast::<u8>(), len) }
}

fn dispatch<H>(state: &mut HandlerState<H>, result: types::Result<()>) -> RdbRes {
    match result {
        Ok(()) => RdbRes_RDB_OK,
        Err(e) => {
            state.last_error = Some(e);
            RdbRes_RDB_ERR_CANCEL_PARSING
        }
    }
}

/// Run `f` inside `catch_unwind`. All type conversion AND handler calls must
/// happen inside `f` so that a panic in any of them is caught.
fn guarded<H, F>(state: &mut HandlerState<H>, f: F) -> RdbRes
where
    F: FnOnce(&mut H) -> types::Result<()>,
{
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| f(&mut state.handler)));
    if let Ok(r) = result {
        dispatch(state, r)
    } else {
        state.last_error = Some(RdbError::Parser {
            code: 0,
            message: "handler panicked".into(),
        });
        RdbRes_RDB_ERR_CANCEL_PARSING
    }
}

// SAFETY (applies to all trampolines below):
// `user_data` is a `Box::into_raw`'d `HandlerState<H>` created in `Parser::new`.
// It remains valid for the lifetime of the `Parser` and is only accessed from
// the single thread that calls `RDB_parse`. `RdbBulk` pointers and struct
// pointers are guaranteed valid by librdb for the duration of the callback.

pub unsafe extern "C" fn trampoline_start_rdb<H: RdbHandlers>(
    _p: *mut RdbParser,
    user_data: *mut c_void,
    rdb_version: i32,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| h.handle_start_rdb(rdb_version))
}

pub unsafe extern "C" fn trampoline_end_rdb<H: RdbHandlers>(
    _p: *mut RdbParser,
    user_data: *mut c_void,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, RdbHandlers::handle_end_rdb)
}

pub unsafe extern "C" fn trampoline_new_db<H: RdbHandlers>(
    _p: *mut RdbParser,
    user_data: *mut c_void,
    dbnum: i32,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| h.handle_new_db(dbnum))
}

pub unsafe extern "C" fn trampoline_db_size<H: RdbHandlers>(
    _p: *mut RdbParser,
    user_data: *mut c_void,
    db_size: u64,
    exp_size: u64,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| h.handle_db_size(db_size, exp_size))
}

pub unsafe extern "C" fn trampoline_slot_info<H: RdbHandlers>(
    _p: *mut RdbParser,
    user_data: *mut c_void,
    info: *mut RdbSlotInfo,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let slot_info = SlotInfo::from_raw(unsafe { &*info });
        h.handle_slot_info(&slot_info)
    })
}

pub unsafe extern "C" fn trampoline_aux_field<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    auxkey: RdbBulk,
    auxval: RdbBulk,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let key = unsafe { bulk_to_slice(p, auxkey) };
        let value = unsafe { bulk_to_slice(p, auxval) };
        h.handle_aux_field(key, value)
    })
}

pub unsafe extern "C" fn trampoline_new_key<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    key: RdbBulk,
    info: *mut RdbKeyInfo,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let key_slice = unsafe { bulk_to_slice(p, key) };
        let key_info = KeyInfo::from_raw(unsafe { &*info })?;
        h.handle_new_key(key_slice, &key_info)
    })
}

pub unsafe extern "C" fn trampoline_end_key<H: RdbHandlers>(
    _p: *mut RdbParser,
    user_data: *mut c_void,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, RdbHandlers::handle_end_key)
}

pub unsafe extern "C" fn trampoline_string_value<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    str_: RdbBulk,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let value = unsafe { bulk_to_slice(p, str_) };
        h.handle_string_value(value)
    })
}

pub unsafe extern "C" fn trampoline_list_item<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    item: RdbBulk,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let item_slice = unsafe { bulk_to_slice(p, item) };
        h.handle_list_item(item_slice)
    })
}

pub unsafe extern "C" fn trampoline_hash_field<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    field: RdbBulk,
    value: RdbBulk,
    expire_at: i64,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let field_slice = unsafe { bulk_to_slice(p, field) };
        let value_slice = unsafe { bulk_to_slice(p, value) };
        h.handle_hash_field(field_slice, value_slice, expire_at)
    })
}

pub unsafe extern "C" fn trampoline_set_member<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    member: RdbBulk,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let member_slice = unsafe { bulk_to_slice(p, member) };
        h.handle_set_member(member_slice)
    })
}

pub unsafe extern "C" fn trampoline_zset_member<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    member: RdbBulk,
    score: f64,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let member_slice = unsafe { bulk_to_slice(p, member) };
        h.handle_zset_member(member_slice, score)
    })
}

pub unsafe extern "C" fn trampoline_function<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    func: RdbBulk,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let func_slice = unsafe { bulk_to_slice(p, func) };
        h.handle_function(func_slice)
    })
}

pub unsafe extern "C" fn trampoline_module<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    module_name: RdbBulk,
    serialized_size: usize,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let name_slice = unsafe { bulk_to_slice(p, module_name) };
        h.handle_module(name_slice, serialized_size)
    })
}

pub unsafe extern "C" fn trampoline_stream_metadata<H: RdbHandlers>(
    _p: *mut RdbParser,
    user_data: *mut c_void,
    meta: *mut RdbStreamMeta,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let stream_meta = RsStreamMeta::from_raw(unsafe { &*meta });
        h.handle_stream_metadata(&stream_meta)
    })
}

pub unsafe extern "C" fn trampoline_stream_item<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    id: *mut RdbStreamID,
    field: RdbBulk,
    value: RdbBulk,
    items_left: i64,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let stream_id = StreamId::from_raw(unsafe { &*id });
        let field_slice = unsafe { bulk_to_slice(p, field) };
        let value_slice = unsafe { bulk_to_slice(p, value) };
        h.handle_stream_item(&stream_id, field_slice, value_slice, items_left)
    })
}

pub unsafe extern "C" fn trampoline_stream_new_cgroup<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    grp_name: RdbBulk,
    meta: *mut RdbStreamGroupMeta,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let name_slice = unsafe { bulk_to_slice(p, grp_name) };
        let group_meta = RsStreamGroupMeta::from_raw(unsafe { &*meta });
        h.handle_stream_new_cgroup(name_slice, &group_meta)
    })
}

pub unsafe extern "C" fn trampoline_stream_cgroup_pending_entry<H: RdbHandlers>(
    _p: *mut RdbParser,
    user_data: *mut c_void,
    pending_entry: *mut RdbStreamPendingEntry,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let entry = RsStreamPendingEntry::from_raw(unsafe { &*pending_entry });
        h.handle_stream_cgroup_pending_entry(&entry)
    })
}

pub unsafe extern "C" fn trampoline_stream_new_consumer<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    cons_name: RdbBulk,
    meta: *mut RdbStreamConsumerMeta,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let name_slice = unsafe { bulk_to_slice(p, cons_name) };
        let consumer_meta = RsStreamConsumerMeta::from_raw(unsafe { &*meta });
        h.handle_stream_new_consumer(name_slice, &consumer_meta)
    })
}

pub unsafe extern "C" fn trampoline_stream_consumer_pending_entry<H: RdbHandlers>(
    _p: *mut RdbParser,
    user_data: *mut c_void,
    stream_id: *mut RdbStreamID,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let id = StreamId::from_raw(unsafe { &*stream_id });
        h.handle_stream_consumer_pending_entry(&id)
    })
}

pub unsafe extern "C" fn trampoline_stream_idmp_meta<H: RdbHandlers>(
    _p: *mut RdbParser,
    user_data: *mut c_void,
    meta: *mut RdbStreamIdmpMeta,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let idmp_meta = RsStreamIdmpMeta::from_raw(unsafe { &*meta });
        h.handle_stream_idmp_meta(&idmp_meta)
    })
}

pub unsafe extern "C" fn trampoline_stream_idmp_producer<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    producer: *mut RdbStreamIdmpProducer,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let raw = unsafe { &*producer };
        let pid = unsafe { bulk_to_slice(p, raw.pid) };
        h.handle_stream_idmp_producer(pid, raw.numEntries)
    })
}

pub unsafe extern "C" fn trampoline_stream_idmp_entry<H: RdbHandlers>(
    p: *mut RdbParser,
    user_data: *mut c_void,
    entry: *mut RdbStreamIdmpEntry,
) -> RdbRes {
    let state = unsafe { &mut *(user_data.cast::<HandlerState<H>>()) };
    guarded(state, |h| {
        let raw = unsafe { &*entry };
        let iid = unsafe { bulk_to_slice(p, raw.iid) };
        let stream_id = StreamId::from_raw(&raw.streamId);
        h.handle_stream_idmp_entry(iid, &stream_id)
    })
}

pub fn build_callbacks<H: RdbHandlers>() -> librdb_sys::RdbHandlersDataCallbacks {
    librdb_sys::RdbHandlersDataCallbacks {
        handleStartRdb: Some(trampoline_start_rdb::<H>),
        handleEndRdb: Some(trampoline_end_rdb::<H>),
        handleNewDb: Some(trampoline_new_db::<H>),
        handleDbSize: Some(trampoline_db_size::<H>),
        handleSlotInfo: Some(trampoline_slot_info::<H>),
        handleAuxField: Some(trampoline_aux_field::<H>),
        handleNewKey: Some(trampoline_new_key::<H>),
        handleEndKey: Some(trampoline_end_key::<H>),
        handleStringValue: Some(trampoline_string_value::<H>),
        handleListItem: Some(trampoline_list_item::<H>),
        handleHashField: Some(trampoline_hash_field::<H>),
        handleSetMember: Some(trampoline_set_member::<H>),
        handleZsetMember: Some(trampoline_zset_member::<H>),
        handleFunction: Some(trampoline_function::<H>),
        handleModule: Some(trampoline_module::<H>),
        handleStreamMetadata: Some(trampoline_stream_metadata::<H>),
        handleStreamItem: Some(trampoline_stream_item::<H>),
        handleStreamNewCGroup: Some(trampoline_stream_new_cgroup::<H>),
        handleStreamCGroupPendingEntry: Some(trampoline_stream_cgroup_pending_entry::<H>),
        handleStreamNewConsumer: Some(trampoline_stream_new_consumer::<H>),
        handleStreamConsumerPendingEntry: Some(trampoline_stream_consumer_pending_entry::<H>),
        handleStreamIdmpMeta: Some(trampoline_stream_idmp_meta::<H>),
        handleStreamIdmpProducer: Some(trampoline_stream_idmp_producer::<H>),
        handleStreamIdmpEntry: Some(trampoline_stream_idmp_entry::<H>),
    }
}
