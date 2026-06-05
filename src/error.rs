//! Typed error handling for Trurl.
//!
//! All operations return [`Result<T>`] with structured [`Error`] variants.
//! Fail-closed on writes, warn on reads.

/// Alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Every failure mode Trurl can encounter.
///
/// Variants are added as features land. Phase 0 carries only the
/// scaffolding variants; real I/O and validation errors arrive in Phase 1.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Command is defined but not yet implemented.
    #[error("{0}")]
    NotImplemented(String),

    /// Filesystem I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML deserialization failure.
    #[error("invalid TOML: {0}")]
    TomlRead(#[from] toml::de::Error),

    /// TOML serialization failure.
    #[error("TOML serialization error: {0}")]
    TomlWrite(#[from] toml::ser::Error),
}
