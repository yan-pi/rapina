//! Testing utilities for Rapina applications.
//!
//! This module provides a test client for integration testing without
//! starting a full HTTP server.

mod client;

pub use client::{TestClient, TestRequestBuilder, TestResponse};
