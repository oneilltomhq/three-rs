//! Ports of `three.js/examples/jsm/utils`, plus the one seam that has no
//! three.js counterpart because in three.js it is the browser: [`time`], which
//! is `performance.now()` and `Date.now()`.

pub mod sort_utils;
pub mod time;

pub use sort_utils::radix_sort;
pub use time::{date_now_ms, now_ms};
