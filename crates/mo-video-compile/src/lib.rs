//! Exact, bounded video projection. Pure authoring computation; no codec, I/O, GPU or host state.
mod contract;
mod effects;
mod index;
mod presentation;
mod project;
pub use contract::*;
pub use presentation::*;
pub use project::CompiledDocument;

use mo_video_model::{ValidationError, ValidationLimits, ValidationReport};

#[derive(Debug, Clone, Copy, Default)]
pub struct CompileLimits {
    pub validation: ValidationLimits,
}

#[derive(Debug, Clone, Copy)]
pub struct QueryLimits {
    pub max_visits: usize,
    /// Active clip occurrences evaluated across all nested instances, including silent stages.
    pub max_nodes: usize,
    pub max_depth: usize,
    /// Final serialized size and a conservative expansion charge before copying nodes/effects/text.
    /// A query can be refused by the expansion charge before reaching this serialized size.
    pub max_output_bytes: usize,
}
impl Default for QueryLimits {
    fn default() -> Self {
        Self {
            max_visits: 1_000_000,
            max_nodes: 4096,
            max_depth: 64,
            max_output_bytes: 32 * 1024 * 1024,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CompileError {
    #[error("invalid video document")]
    Invalid(ValidationReport),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Time(#[from] mo_time::TimeError),
    #[error(transparent)]
    Canonical(#[from] mo_common::CanonicalError),
    #[error("video computation cancelled")]
    Cancelled,
    #[error("video computation exceeds its budget")]
    Budget,
    #[error("invalid video query limits")]
    Limits,
    #[error("sample index is outside the project")]
    Outside,
    #[error("invalid source presentation index: {0}")]
    Presentation(&'static str),
    #[error("validated video invariant failed: {0}")]
    Invariant(&'static str),
}
pub type Result<T> = std::result::Result<T, CompileError>;

pub(crate) struct Work<'a> {
    limits: QueryLimits,
    check: &'a dyn Fn() -> bool,
    stats: QueryStats,
}
impl<'a> Work<'a> {
    fn new(limits: QueryLimits, check: &'a dyn Fn() -> bool) -> Result<Self> {
        if limits.max_depth == 0
            || limits.max_depth > 128
            || limits.max_visits == 0
            || limits.max_nodes == 0
            || limits.max_output_bytes == 0
        {
            return Err(CompileError::Limits);
        }
        let mut work = Self {
            limits,
            check,
            stats: QueryStats::default(),
        };
        work.visit()?;
        Ok(work)
    }
    fn visit(&mut self) -> Result<()> {
        if (self.check)() {
            return Err(CompileError::Cancelled);
        }
        if self.stats.visited >= self.limits.max_visits {
            return Err(CompileError::Budget);
        }
        self.stats.visited += 1;
        Ok(())
    }
    fn emit(&mut self, depth: usize) -> Result<()> {
        self.visit()?;
        if self.stats.selected >= self.limits.max_nodes || depth > self.limits.max_depth {
            return Err(CompileError::Budget);
        }
        self.stats.selected += 1;
        Ok(())
    }
    fn bytes(&mut self, n: usize) -> Result<()> {
        self.limits.max_output_bytes = self
            .limits
            .max_output_bytes
            .checked_sub(n)
            .ok_or(CompileError::Budget)?;
        Ok(())
    }
}
