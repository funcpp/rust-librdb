#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unsafe_code)]
#![allow(unused_results)] // bindgen layout tests produce unused &str results
#![allow(clippy::pedantic)]

include!("bindings.rs");

/// `RDB_ARRAY_INSERT_IDX_NONE` — the "no insert cursor" sentinel for `RDB_TYPE_ARRAY`.
///
/// Declared by hand instead of via bindgen: the C macro is
/// `#define RDB_ARRAY_INSERT_IDX_NONE UINT64_MAX`, but bindgen mis-evaluates the
/// all-bits-set value as `i32 = -1`. The upstream type is `uint64_t`, so the var
/// is blocklisted in `update-bindings.sh` and re-declared here with correct width.
pub const RDB_ARRAY_INSERT_IDX_NONE: u64 = u64::MAX;
