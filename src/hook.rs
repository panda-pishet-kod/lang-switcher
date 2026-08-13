//! Installing and removing the LL hook, the callback, filtering out the program's own
//! injected input, the hook watchdog.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC. Stub: the
//! implementation belongs to stage E3.
//!
//! Requirements this module will cover: FR-01, FR-02, FR-03, FR-08, FR-95, FR-96, FR-97,
//! FR-98, FR-99, NFR-01, NFR-02, NFR-03, NFR-04, NFR-05.
//! Moved out to match the backlog (decision R-17): FR-05, FR-06 (decoding through
//! `ToUnicodeEx`) and FR-13 to module `buffer`, tasks T-03-2 and T-03-3; FR-80 (the hook
//! watchdog) to module `watchdog`, task T-06-2.
//! Implemented by backlog tasks: T-03-1.
