//! Domain Services
//!
//! Pure business logic services that operate on domain entities.
//! These services have no I/O dependencies and are easily testable.

mod orphan_detector;
mod planner;

pub use orphan_detector::{
    extract_path_from_key, has_calvin_signature, OrphanDetectionResult, OrphanDetector, OrphanFile,
    CALVIN_SIGNATURES,
};
pub use planner::{ConflictReason, FileAction, PlannedFile, Planner, SyncPlan, TargetFileState};
