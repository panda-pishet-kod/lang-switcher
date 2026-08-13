//! The SEC-04a debug control channel: a named pipe reporting program state.
//!
//! The single documented exception to SEC-04, and the reason it exists is that SEC-06 is
//! otherwise not provable by automated means: the contents of a password field are not
//! reachable from outside by the design of Windows.
//!
//! Compiled only under the `testing` feature, which is absent from the Release
//! configuration. Metadata only, read only, and restricted to the owner of the current
//! session. Stub: the implementation belongs to task T-03-4.
//!
//! Requirements this module will cover: SEC-04a, the acceptance bench of section 11.5 and
//! acceptance criterion 8 of section 13 of SPEC.
//! Implemented by backlog tasks: T-03-4.
