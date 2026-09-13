//! Typed shared primitives for the Considered design-reasoning engine.
//!
//! `considered-core` defines the domain types that cross crate boundaries:
//! severities, findings, counts, command status, relative artifact paths,
//! and structured errors. It never prints to stdout/stderr or terminates
//! the process — that is the CLI crate's responsibility.

pub mod cache;
pub mod context;
pub mod contract;
pub mod error;
pub mod finding;
pub mod path;
pub mod roll;
pub mod rules;
pub mod state;
pub mod status;
pub mod validate;
pub mod validate_skill;

pub use cache::{hash_corpus, hash_file_content, CacheEntry, CacheKey};
pub use context::{build_manifest, ContextManifest};
pub use contract::{validate_contract, ContractDetails};
pub use error::EngineError;
pub use finding::{Counts, Finding, Severity};
pub use path::ArtifactPath;
pub use roll::{draw_chained, generation_capacity, load_deck, new_key, RollOutput};
pub use rules::{
    evaluate_gate, load_gate_policy, load_rule_catalog, severity_for, GatePolicy, GateResult,
    ReviewInput, RuleCatalog,
};
pub use state::{
    all_runs, check_artifacts, compute_status, discover_surfaces, latest_run, write_check_artifact,
    write_run_record, ArtifactRecord, ReviewValidity, RunRecord, Stage, SurfaceStatus,
};
pub use status::CommandStatus;
pub use validate::{validate_assets, ValidateResult};
pub use validate_skill::{validate_skill, CheckResult, SkillValidationResult};
