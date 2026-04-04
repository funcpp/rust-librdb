#![allow(clippy::needless_collect, clippy::option_option)]

use std::path::PathBuf;

use librdb::{
    DataType, KeyInfo, Parser, RdbError, RdbHandlers, Result, SlotInfo, StreamGroupMeta, StreamId,
    StreamIdmpMeta, StreamMeta, StreamPendingEntry,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../librdb-sys/librdb/test/dumps")
        .join(name)
}

#[derive(Debug, Clone, PartialEq)]
enum Event {
    StartRdb(i32),
    EndRdb,
    NewDb(i32),
    DbSize {
        db_size: u64,
        expires_size: u64,
    },
    SlotInfo(u64),
    AuxField {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    NewKey {
        key: Vec<u8>,
        data_type: DataType,
    },
    EndKey,
    StringValue(Vec<u8>),
    ListItem(Vec<u8>),
    HashField {
        field: Vec<u8>,
        value: Vec<u8>,
        expire_at: i64,
    },
    SetMember(Vec<u8>),
    ZsetMember {
        member: Vec<u8>,
        score: f64,
    },
    Function(Vec<u8>),
    StreamMetadata {
        length: u64,
    },
    StreamItem {
        id: StreamId,
        field: Vec<u8>,
        value: Vec<u8>,
    },
    StreamNewCGroup(Vec<u8>),
    StreamCGroupPendingEntry(StreamId),
    StreamNewConsumer(Vec<u8>),
    StreamConsumerPendingEntry(StreamId),
    StreamIdmpMeta {
        duration: u64,
    },
    StreamIdmpProducer {
        pid: Vec<u8>,
        num_entries: u64,
    },
    StreamIdmpEntry {
        iid: Vec<u8>,
        stream_id: StreamId,
    },
}

#[derive(Default)]
struct Collector {
    events: Vec<Event>,
}

impl RdbHandlers for Collector {
    fn handle_start_rdb(&mut self, rdb_version: i32) -> Result<()> {
        self.events.push(Event::StartRdb(rdb_version));
        Ok(())
    }

    fn handle_end_rdb(&mut self) -> Result<()> {
        self.events.push(Event::EndRdb);
        Ok(())
    }

    fn handle_new_db(&mut self, db_number: i32) -> Result<()> {
        self.events.push(Event::NewDb(db_number));
        Ok(())
    }

    fn handle_db_size(&mut self, db_size: u64, expires_size: u64) -> Result<()> {
        self.events.push(Event::DbSize {
            db_size,
            expires_size,
        });
        Ok(())
    }

    fn handle_slot_info(&mut self, info: &SlotInfo) -> Result<()> {
        self.events.push(Event::SlotInfo(info.slot_id));
        Ok(())
    }

    fn handle_aux_field(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.events.push(Event::AuxField {
            key: key.to_vec(),
            value: value.to_vec(),
        });
        Ok(())
    }

    fn handle_new_key(&mut self, key: &[u8], info: &KeyInfo) -> Result<()> {
        self.events.push(Event::NewKey {
            key: key.to_vec(),
            data_type: info.data_type,
        });
        Ok(())
    }

    fn handle_end_key(&mut self) -> Result<()> {
        self.events.push(Event::EndKey);
        Ok(())
    }

    fn handle_string_value(&mut self, value: &[u8]) -> Result<()> {
        self.events.push(Event::StringValue(value.to_vec()));
        Ok(())
    }

    fn handle_list_item(&mut self, item: &[u8]) -> Result<()> {
        self.events.push(Event::ListItem(item.to_vec()));
        Ok(())
    }

    fn handle_hash_field(&mut self, field: &[u8], value: &[u8], expire_at: i64) -> Result<()> {
        self.events.push(Event::HashField {
            field: field.to_vec(),
            value: value.to_vec(),
            expire_at,
        });
        Ok(())
    }

    fn handle_set_member(&mut self, member: &[u8]) -> Result<()> {
        self.events.push(Event::SetMember(member.to_vec()));
        Ok(())
    }

    fn handle_zset_member(&mut self, member: &[u8], score: f64) -> Result<()> {
        self.events.push(Event::ZsetMember {
            member: member.to_vec(),
            score,
        });
        Ok(())
    }

    fn handle_function(&mut self, func: &[u8]) -> Result<()> {
        self.events.push(Event::Function(func.to_vec()));
        Ok(())
    }

    fn handle_stream_metadata(&mut self, meta: &StreamMeta) -> Result<()> {
        self.events.push(Event::StreamMetadata {
            length: meta.length,
        });
        Ok(())
    }

    fn handle_stream_item(
        &mut self,
        id: &StreamId,
        field: &[u8],
        value: &[u8],
        _items_left: i64,
    ) -> Result<()> {
        self.events.push(Event::StreamItem {
            id: *id,
            field: field.to_vec(),
            value: value.to_vec(),
        });
        Ok(())
    }

    fn handle_stream_new_cgroup(&mut self, name: &[u8], _: &StreamGroupMeta) -> Result<()> {
        self.events.push(Event::StreamNewCGroup(name.to_vec()));
        Ok(())
    }

    fn handle_stream_cgroup_pending_entry(&mut self, entry: &StreamPendingEntry) -> Result<()> {
        self.events.push(Event::StreamCGroupPendingEntry(entry.id));
        Ok(())
    }

    fn handle_stream_new_consumer(
        &mut self,
        name: &[u8],
        _: &librdb::StreamConsumerMeta,
    ) -> Result<()> {
        self.events.push(Event::StreamNewConsumer(name.to_vec()));
        Ok(())
    }

    fn handle_stream_consumer_pending_entry(&mut self, id: &StreamId) -> Result<()> {
        self.events.push(Event::StreamConsumerPendingEntry(*id));
        Ok(())
    }

    fn handle_stream_idmp_meta(&mut self, meta: &StreamIdmpMeta) -> Result<()> {
        self.events.push(Event::StreamIdmpMeta {
            duration: meta.duration,
        });
        Ok(())
    }

    fn handle_stream_idmp_producer(&mut self, pid: &[u8], num_entries: u64) -> Result<()> {
        self.events.push(Event::StreamIdmpProducer {
            pid: pid.to_vec(),
            num_entries,
        });
        Ok(())
    }

    fn handle_stream_idmp_entry(&mut self, iid: &[u8], stream_id: &StreamId) -> Result<()> {
        self.events.push(Event::StreamIdmpEntry {
            iid: iid.to_vec(),
            stream_id: *stream_id,
        });
        Ok(())
    }
}

fn parse_collect(rdb: &str) -> Vec<Event> {
    let mut parser = Parser::new(Collector::default()).expect("create parser");
    parser.parse_file(fixture(rdb)).expect("parse");
    parser.into_handler().events
}

fn keys_of(events: &[Event]) -> Vec<&[u8]> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::NewKey { key, .. } => Some(key.as_slice()),
            _ => None,
        })
        .collect()
}

#[test]
fn string_single_key() {
    let events = parse_collect("single_key.rdb");
    let keys = keys_of(&events);
    assert_eq!(keys, vec![b"xxx"]);
    assert!(events.contains(&Event::NewKey {
        key: b"xxx".to_vec(),
        data_type: DataType::String,
    }));
    assert!(events.contains(&Event::StringValue(b"111".to_vec())));
}

#[test]
fn list_multiple() {
    let events = parse_collect("multiple_lists_strings.rdb");

    let list_keys: Vec<&[u8]> = events
        .iter()
        .filter_map(|e| match e {
            Event::NewKey {
                key,
                data_type: DataType::List,
            } => Some(key.as_slice()),
            _ => None,
        })
        .collect();
    assert_eq!(list_keys.len(), 3, "expected 3 list keys (mylist1/2/3)");

    let list_items: Vec<&[u8]> = events
        .iter()
        .filter_map(|e| match e {
            Event::ListItem(v) => Some(v.as_slice()),
            _ => None,
        })
        .collect();
    // mylist1: [v1], mylist2: [v2,v1], mylist3: [v3,v2,v1] = 6 items
    assert_eq!(list_items.len(), 6, "expected 6 total list items");
}

#[test]
fn hash_fields() {
    let events = parse_collect("hash_lp_v11.rdb");
    let hash_fields: Vec<(&[u8], &[u8])> = events
        .iter()
        .filter_map(|e| match e {
            Event::HashField { field, value, .. } => Some((field.as_slice(), value.as_slice())),
            _ => None,
        })
        .collect();
    // hash1: 5 fields + hash2: 6 fields = 11
    assert_eq!(hash_fields.len(), 11);
}

#[test]
fn hash_field_expiry() {
    let events = parse_collect("hash_with_expire_v12.rdb");
    let fields: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            Event::HashField {
                field, expire_at, ..
            } => Some((field.clone(), *expire_at)),
            _ => None,
        })
        .collect();
    // myhash: field1, field2, field3
    assert_eq!(fields.len(), 3);
    assert!(
        fields.iter().any(|(_, exp)| *exp != -1),
        "expected at least one field with a non-negative expiry"
    );
}

#[test]
fn set_members() {
    let events = parse_collect("plain_set_v6.rdb");
    let members: Vec<&[u8]> = events
        .iter()
        .filter_map(|e| match e {
            Event::SetMember(v) => Some(v.as_slice()),
            _ => None,
        })
        .collect();
    assert_eq!(members.len(), 10);
}

#[test]
fn zset_members_with_scores() {
    let events = parse_collect("plain_zset_v6.rdb");
    let members: Vec<(&[u8], f64)> = events
        .iter()
        .filter_map(|e| match e {
            Event::ZsetMember { member, score } => Some((member.as_slice(), *score)),
            _ => None,
        })
        .collect();
    assert_eq!(members.len(), 24);

    let has_inf = members.iter().any(|(_, s)| *s == f64::INFINITY);
    let has_neg_inf = members.iter().any(|(_, s)| *s == f64::NEG_INFINITY);
    assert!(has_inf, "expected +inf score");
    assert!(has_neg_inf, "expected -inf score");
}

#[test]
fn stream_entries() {
    let events = parse_collect("stream_v11.rdb");
    assert!(events.iter().any(|e| matches!(
        e,
        Event::NewKey {
            data_type: DataType::Stream,
            ..
        }
    )));

    let meta_events: Vec<u64> = events
        .iter()
        .filter_map(|e| match e {
            Event::StreamMetadata { length } => Some(*length),
            _ => None,
        })
        .collect();
    assert_eq!(meta_events, vec![6]);

    let item_count = events
        .iter()
        .filter(|e| matches!(e, Event::StreamItem { .. }))
        .count();
    // 6 entries: 5 with 1 field ("message") + 1 with 3 fields = 8 field-value pairs
    assert_eq!(item_count, 8);
}

#[test]
fn multiple_databases() {
    let events = parse_collect("multiple_dbs.rdb");
    let dbs: Vec<i32> = events
        .iter()
        .filter_map(|e| match e {
            Event::NewDb(n) => Some(*n),
            _ => None,
        })
        .collect();
    assert!(dbs.contains(&0));
    assert!(dbs.contains(&1));
    assert!(dbs.contains(&2));

    let keys = keys_of(&events);
    assert!(keys.contains(&b"x".as_slice()));
    assert!(keys.contains(&b"y".as_slice()));
    assert!(keys.contains(&b"z".as_slice()));
}

#[test]
fn callback_ordering() {
    let events = parse_collect("single_key.rdb");

    let new_key_pos = events
        .iter()
        .position(|e| matches!(e, Event::NewKey { .. }))
        .expect("expected NewKey");
    let string_val_pos = events
        .iter()
        .position(|e| matches!(e, Event::StringValue(_)))
        .expect("expected StringValue");
    let end_key_pos = events
        .iter()
        .position(|e| matches!(e, Event::EndKey))
        .expect("expected EndKey");

    assert!(
        new_key_pos < string_val_pos,
        "NewKey must precede StringValue"
    );
    assert!(
        string_val_pos < end_key_pos,
        "StringValue must precede EndKey"
    );
}

#[test]
fn callback_ordering_multi_key() {
    let events = parse_collect("multiple_lists_strings.rdb");

    let mut inside_key = false;
    for event in &events {
        match event {
            Event::NewKey { .. } => {
                assert!(!inside_key, "NewKey without preceding EndKey");
                inside_key = true;
            }
            Event::EndKey => {
                assert!(inside_key, "EndKey without preceding NewKey");
                inside_key = false;
            }
            Event::StringValue(_)
            | Event::ListItem(_)
            | Event::HashField { .. }
            | Event::SetMember(_)
            | Event::ZsetMember { .. } => {
                assert!(inside_key, "value callback outside NewKey/EndKey pair");
            }
            _ => {}
        }
    }
    assert!(!inside_key, "missing final EndKey");
}

#[test]
fn expiry_none_when_not_set() {
    struct ExpiryChecker {
        expire: Option<Option<i64>>,
    }
    impl RdbHandlers for ExpiryChecker {
        fn handle_new_key(&mut self, _key: &[u8], info: &KeyInfo) -> Result<()> {
            self.expire = Some(info.expire_time_ms);
            Ok(())
        }
    }

    let mut parser = Parser::new(ExpiryChecker { expire: None }).expect("create parser");
    parser.parse_file(fixture("single_key.rdb")).expect("parse");
    let checker = parser.into_handler();
    assert_eq!(checker.expire, Some(None), "expected no expiry");
}

#[test]
fn lru_metadata_present() {
    struct LruChecker {
        lru_idle: Option<Option<i64>>,
    }
    impl RdbHandlers for LruChecker {
        fn handle_new_key(&mut self, _key: &[u8], info: &KeyInfo) -> Result<()> {
            self.lru_idle = Some(info.lru_idle);
            Ok(())
        }
    }

    let mut parser = Parser::new(LruChecker { lru_idle: None }).expect("create parser");
    parser
        .parse_file(fixture("mem_policy_lru.rdb"))
        .expect("parse");
    let checker = parser.into_handler();
    assert!(
        matches!(checker.lru_idle, Some(Some(_))),
        "expected Some(Some(idle)) for LRU fixture"
    );
}

#[test]
fn lru_metadata_absent() {
    struct LruChecker {
        lru_idle: Option<Option<i64>>,
    }
    impl RdbHandlers for LruChecker {
        fn handle_new_key(&mut self, _key: &[u8], info: &KeyInfo) -> Result<()> {
            self.lru_idle = Some(info.lru_idle);
            Ok(())
        }
    }

    let mut parser = Parser::new(LruChecker { lru_idle: None }).expect("create parser");
    parser.parse_file(fixture("single_key.rdb")).expect("parse");
    let checker = parser.into_handler();
    assert_eq!(checker.lru_idle, Some(None), "single_key has no LRU policy");
}

#[test]
fn handler_error_propagates() {
    struct FailOnSecondKey {
        count: usize,
    }
    impl RdbHandlers for FailOnSecondKey {
        fn handle_new_key(&mut self, _key: &[u8], _info: &KeyInfo) -> Result<()> {
            self.count += 1;
            if self.count >= 2 {
                return Err(RdbError::Handler("deliberate error".into()));
            }
            Ok(())
        }
    }

    let mut parser = Parser::new(FailOnSecondKey { count: 0 }).expect("create parser");
    let err = parser
        .parse_file(fixture("multiple_lists_strings.rdb"))
        .unwrap_err();
    assert!(
        err.to_string().contains("deliberate error"),
        "expected handler error, got: {err}"
    );
}

#[test]
fn early_termination_no_further_callbacks() {
    struct StopAfterFirst {
        key_count: usize,
        value_after_stop: bool,
    }
    impl RdbHandlers for StopAfterFirst {
        fn handle_new_key(&mut self, _key: &[u8], _info: &KeyInfo) -> Result<()> {
            self.key_count += 1;
            if self.key_count > 1 {
                self.value_after_stop = true;
            }
            if self.key_count == 1 {
                return Err(RdbError::Handler("stop".into()));
            }
            Ok(())
        }
    }

    let mut parser = Parser::new(StopAfterFirst {
        key_count: 0,
        value_after_stop: false,
    })
    .expect("create parser");
    let _ = parser.parse_file(fixture("multiple_lists_strings.rdb"));
    let h = parser.into_handler();

    assert_eq!(h.key_count, 1, "only one key should have been seen");
    assert!(!h.value_after_stop, "no callbacks after Err");
}

#[test]
fn handler_accessible_after_failure() {
    struct Counter {
        count: usize,
    }
    impl RdbHandlers for Counter {
        fn handle_new_key(&mut self, _key: &[u8], _info: &KeyInfo) -> Result<()> {
            self.count += 1;
            if self.count == 3 {
                return Err(RdbError::Handler("stop at 3".into()));
            }
            Ok(())
        }
    }

    let mut parser = Parser::new(Counter { count: 0 }).expect("create parser");
    let err = parser.parse_file(fixture("multiple_lists_strings.rdb"));
    assert!(err.is_err());
    assert_eq!(
        parser.handler().count,
        3,
        "handler state accessible after failure"
    );
}

#[test]
fn handler_panic_is_caught() {
    struct PanicHandler;
    impl RdbHandlers for PanicHandler {
        fn handle_new_key(&mut self, _key: &[u8], _info: &KeyInfo) -> Result<()> {
            panic!("intentional panic in handler");
        }
    }

    let mut parser = Parser::new(PanicHandler).expect("create parser");
    let err = parser.parse_file(fixture("single_key.rdb")).unwrap_err();
    assert!(
        err.to_string().contains("panicked"),
        "expected panic error, got: {err}"
    );
}

#[test]
fn parse_file_twice_returns_error() {
    let mut parser = Parser::new(Collector::default()).expect("create parser");
    parser
        .parse_file(fixture("empty.rdb"))
        .expect("first parse");

    let err = parser.parse_file(fixture("empty.rdb")).unwrap_err();
    assert!(
        err.to_string().contains("already used"),
        "expected 'already used' error, got: {err}"
    );
}

#[test]
fn nonexistent_file_returns_error() {
    let mut parser = Parser::new(Collector::default()).expect("create parser");
    let err = parser.parse_file("/nonexistent/path.rdb").unwrap_err();
    assert!(
        err.to_string().contains("open") || err.to_string().contains("No such file"),
        "expected file-not-found error, got: {err}"
    );
}

#[test]
fn empty_rdb() {
    let events = parse_collect("empty.rdb");
    assert!(events.contains(&Event::StartRdb(11)));
    assert!(events.contains(&Event::EndRdb));
    assert!(keys_of(&events).is_empty());
}

#[test]
fn parse_fd_single_key() {
    use std::fs::File;

    let file = File::open(fixture("single_key.rdb")).expect("open fixture");

    let mut parser = Parser::new(Collector::default()).expect("create parser");
    parser.parse_fd(file.into()).expect("parse");
    let events = parser.into_handler().events;

    let keys = keys_of(&events);
    assert_eq!(keys, vec![b"xxx"]);
    assert!(events.contains(&Event::StringValue(b"111".to_vec())));
}
