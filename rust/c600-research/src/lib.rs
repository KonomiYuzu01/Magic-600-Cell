//! The research host of 600 Cell Script (`docs/progress/1.0/language-lp5-plan.md`, pieces R8 to R10).
//!
//! Its rows read and check; none chooses, ranks or applies a solving word, and every searching entry stays a research
//! tier candidate that a solving session refuses (the human-solve boundary). Only a KN6 checker's own report writes the
//! independent-checker value (EVD.64).
//!
//! This crate is a scaffold until piece R8 lands its job adapter.

#![forbid(unsafe_code)]
