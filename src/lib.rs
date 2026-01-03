//! ThoughtGate - High-performance sidecar proxy library for governing AI traffic.
//!
//! This library provides the core proxy service, error handling, and logging
//! functionality for the ThoughtGate sidecar proxy.
//!
//! # Traceability
//! - Implements: REQ-CORE-001 (Zero-Copy Peeking Strategy)

pub mod error;
pub mod logging_layer;
pub mod proxy_service;

