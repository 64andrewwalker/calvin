//! Domain Layer
//!
//! This is the core of Calvin - pure business logic without I/O dependencies.
//!
//! ## Structure
//!
//! - `entities/` - Core domain entities (Asset, OutputFile, Lockfile)
//! - `value_objects/` - Immutable value types (Scope, Target, Hash)
//! - `services/` - Domain services (Compiler, Planner, OrphanDetector)
//! - `policies/` - Business rules (ScopePolicy, SecurityPolicy)
//! - `ports/` - Interface definitions for infrastructure
//!
//! ## Design Principles
//!
//! 1. **No I/O** - This layer never touches the file system or network directly
//! 2. **Pure Functions** - Services are stateless and testable
//! 3. **Ports & Adapters** - All I/O goes through trait-defined ports

pub mod entities;
pub mod ports;
pub mod value_objects;

// TODO: Add these modules as we extract them
// pub mod services;
// pub mod policies;
