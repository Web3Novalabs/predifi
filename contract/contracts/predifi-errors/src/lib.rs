#![no_std]

//! # PrediFi Errors
//!
//! This crate provides a comprehensive error handling system for PrediFi smart contracts.
//!
//! ## Features
//!
//! - **Granular Error Codes**: Specific error variants for validation failures, arithmetic
//!   overflows, state inconsistencies, and more
//! - **Gap-Based Numbering**: Error codes are organized in ranges (e.g., 1-5 for initialization,
//!   10-15 for authorization) allowing future additions without breaking existing mappings
//! - **Error Categorization**: Errors are grouped into logical categories for better organization
//! - **Frontend-Friendly**: Includes helper methods for error codes, categories, and recoverability
//! - **Display Implementation**: Human-readable error messages for logging and debugging
//!
//! ## Error Categories
//!
//! - **Initialization (1-5)**: Contract setup and configuration errors
//! - **Authorization (10-15)**: Access control and permission errors
//! - **Pool State (20-30)**: Pool lifecycle and state management errors
//! - **Prediction (40-50)**: Betting and prediction placement errors
//! - **Claiming (60-70)**: Reward claiming errors
//! - **Timestamp (80-85)**: Time validation errors
//! - **Validation (90-100)**: General data validation errors
//! - **Arithmetic (110-118)**: Mathematical operation errors
//! - **Storage (120-129)**: Data persistence and consistency errors
//! - **Granular Validation (130-145)**: Specific input validation errors
//! - **Token (150-159)**: Token transfer and interaction errors
//! - **Oracle (160-169)**: Oracle and resolution errors
//! - **Reward (170-179)**: Reward calculation errors
//! - **Admin (180-189)**: Emergency and administrative errors
//! - **Rate Limiting (190-199)**: Spam prevention errors
//!
//! ## Usage Example
//!
//! ```rust,ignore
//! use predifi_errors::PrediFiError;
//!
//! fn validate_amount(amount: i128) -> Result<(), PrediFiError> {
//!     if amount <= 0 {
//!         return Err(PrediFiError::AmountIsZero);
//!     }
//!     Ok(())
//! }
//!
//! // Get error details
//! let error = PrediFiError::AmountIsZero;
//! let code = error.code(); // 130
//! let category = error.category(); // "granular_validation"
//! let message = error.as_str(); // "Amount cannot be zero"
//! let recoverable = error.is_recoverable(); // true
//! ```

pub mod errors;

pub use errors::PrediFiError;
