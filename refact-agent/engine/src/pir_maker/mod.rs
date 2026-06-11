//! PIR_maker — live Project Intelligence & Representation for ESP-IDF firmware.

pub mod agent;
pub mod analyzer;
pub mod apply_patch;
pub mod board_validate;
pub mod builder;
pub mod diagram_view_builders;
pub mod diagrams;
pub mod persistence;
pub mod schema;
pub mod service;
pub mod session;
pub mod sync;
pub mod validation_lock;

pub use schema::{PirAnalyzeResult, PirApprovalStatus, PirDocument, TopologyDiff, PIR_SCHEMA_VERSION};
pub use service::{
    agent_context_for_chat, analyze_for_chat, apply_node_patch, apply_structural_patch, approve,
    compact_summary_for_agent, project_has_codegen_artifacts, spawn_analyze_for_chat,
};
pub use builder::diff_documents;
pub use session::{get, PirSessionStatus};
