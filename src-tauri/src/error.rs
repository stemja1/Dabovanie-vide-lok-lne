//! Centralized application error type for Tauri commands.
//!
//! Before this module existed, every command returned `Result<T, String>` and
//! built the error with `.map_err(|e| e.to_string())`. That has two real costs:
//!
//! 1. `anyhow::Error::to_string()` only prints the *outermost* message in the
//!    context chain built by `.with_context(...)` — any deeper cause (e.g. the
//!    actual WSL/`bash` failure underneath a "Failed to run wizard step"
//!    wrapper) was silently dropped before it ever reached the user or the logs.
//! 2. The frontend received an undifferentiated string for every failure mode
//!    (validation, I/O, a poisoned config mutex, a WSL subprocess failure),
//!    so it could never distinguish "your input was invalid" from "the
//!    background process crashed" without parsing message text.
//!
//! `AppError` keeps the wire format identical (it serializes to a plain
//! string, exactly like the old `Result<T, String>`, so no frontend change is
//! required) while giving the Rust side a real enum to match on, and printing
//! the *full* anyhow context chain instead of just the top frame.
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// A `std::sync::Mutex` guarding shared state (currently only the
    /// `AppConfig`) was poisoned by a prior panic while the lock was held.
    /// The process is still running, so we surface a clear, actionable
    /// message instead of the low-level `PoisonError` internals.
    #[error("Interný stav aplikácie je poškodený (poisoned mutex) — reštartujte aplikáciu")]
    LockPoisoned,

    /// A caller-supplied argument failed validation before any I/O or
    /// subprocess work was attempted (e.g. an unknown wizard step id).
    #[error("{0}")]
    Validation(String),

    /// Catch-all for `anyhow::Error` coming out of the WSL/pipeline/wizard
    /// layers. `{0:#}` (alternate `Display`) prints the full `.with_context`
    /// chain — "outer cause: middle cause: root cause" — instead of just the
    /// outermost frame that `.to_string()` used to show.
    #[error("{0:#}")]
    Internal(#[from] anyhow::Error),

    /// Plain filesystem errors (config load/save) that didn't go through
    /// `anyhow::Context` and so don't already carry a chain.
    #[error("Chyba I/O: {0}")]
    Io(#[from] std::io::Error),
}

/// Tauri commands may return any `E: Serialize` as their error type (it does
/// not have to be `String`); the JS side of `invoke()` still just sees the
/// rejected value, so serializing as a plain string keeps today's frontend
/// error handling (which expects a string message) working unchanged.
impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_variant_prints_full_anyhow_chain() {
        let root = anyhow::anyhow!("bash exited with status 1");
        let wrapped = root.context("nepodarilo sa spustiť krok inštalácie");
        let app_err: AppError = wrapped.into();
        let message = app_err.to_string();

        // Both the outer context AND the root cause must be visible — this is
        // exactly what plain `.to_string()` on the old `anyhow::Error` path
        // used to lose.
        assert!(message.contains("nepodarilo sa spustiť krok inštalácie"));
        assert!(message.contains("bash exited with status 1"));
    }

    #[test]
    fn validation_variant_is_verbatim() {
        let err = AppError::Validation("Neznámy krok: xyz".to_string());
        assert_eq!(err.to_string(), "Neznámy krok: xyz");
    }

    #[test]
    fn serializes_as_plain_json_string_for_frontend_compat() {
        let err = AppError::Validation("bad input".to_string());
        let json = serde_json::to_string(&err).expect("serialize");
        assert_eq!(json, "\"bad input\"");
    }
}
