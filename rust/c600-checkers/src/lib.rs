//! KN6's independent checkers (chapter 01 section 7; `docs/progress/1.0/language-lp5-plan.md`, pieces R2 to R6).
//!
//! Each checker is kernel code identified by its source digest (KN6.3). It recomputes letter by letter on the exact
//! engines without caches and shares no search or construction code with a producer (KN6.2). The research-archive kind
//! is a family of sub-checkers, one per claim family, each with its own identity and mutation gate (the plan's reading
//! A, adopted on 23 November 2026).
//!
//! This crate is a scaffold until piece R2 lands its checker core.

#![forbid(unsafe_code)]
