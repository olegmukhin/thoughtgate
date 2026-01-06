//! ThoughtGate - High-performance sidecar proxy library for governing AI traffic.
//!
//! This library provides the core proxy service, error handling, and logging
//! functionality for the ThoughtGate sidecar proxy.
//!
//! # Traffic Paths
//!
//! ThoughtGate implements three traffic paths based on governance decisions:
//!
//! - **Green Path (REQ-CORE-001):** Zero-copy streaming for trusted traffic.
//! - **Amber Path (REQ-CORE-002):** Buffered inspection for validation.
//! - **Red Path:** Immediate rejection.
//!
//! # Traceability
//! - Implements: REQ-CORE-001 (Zero-Copy Peeking Strategy)
//! - Implements: REQ-CORE-002 (Buffered Termination Strategy)

pub mod buffered_forwarder;
pub mod config;
pub mod error;
pub mod inspector;
pub mod logging_layer;
pub mod metrics;
pub mod proxy_body;
pub mod proxy_service;
pub mod timeout;
