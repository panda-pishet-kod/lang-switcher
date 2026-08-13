//! Pure transcoding logic. Does not depend on Win32, fully covered by unit tests.
//!
//! Responsibility taken from the module table in section 6.2 of SPEC. Stub: the
//! implementation belongs to stage E2, together with the unit test set of section 11.1.
//!
//! Requirements this module will cover: FR-22, FR-23, FR-24, FR-25, FR-26.
//! Moved out to match the backlog (decision R-17): FR-32 (cycle accuracy, the position
//! counter) to module `layouts`, task T-05-2.
//! Implemented by backlog tasks: T-02-2.
