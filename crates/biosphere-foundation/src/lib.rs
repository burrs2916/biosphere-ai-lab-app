pub mod ontology;
pub mod world;
pub mod topology;
pub mod conditions;
pub mod projection;
pub mod invariants;
pub mod temporal;
pub mod input;
pub mod entity;
pub mod runtime;
pub mod error;
pub mod recovery;
pub mod manifest;
pub mod perception;

pub use ontology::{
    BasicEnvironment,
    BasicEnvironmentState,
    BasicEmbodiment,
    BasicPerception,
    BasicRepresentation,
    BasicExistence,
};
pub use world::{BasicWorld, WorldClock, WorldRules};
pub use topology::StableTopology;
pub use conditions::SensedConditions;
pub use projection::{Projection, AsciiView, TimelineView, TimelineViewModel};
pub use invariants::{WorldAxioms, AxiomConfig};
pub use temporal::{
    StateSnapshot,
    StatePayload,
    StateHistory,
    StateStore,
    StateProvider,
    StateQuery,
    RelationChange,
    RelationChangeKind,
    RelationHistory,
    RelationQuery,
    RelationStore,
    LazyQueryIterator,
    LazyRelationQueryIterator,
    WindowedQuery,
    WindowedRelationQuery,
};
pub use input::{
    ConditionInput,
    InputManager,
    Command,
};
pub use entity::{
    Entity,
    EntityFilter,
    EntityManager,
    EntityQuery,
};
pub use runtime::WorldRuntime;
pub use error::{FoundationError, FoundationResult};
pub use recovery::{RecoveryStrategy, RecoveryResult};
pub use manifest::{
    Manifest,
    ManifestNode,
    NodeKind,
    Value,
    Derivation,
    ManifestDiff,
    DiffOperation,
};
pub use perception::{
    Perception,
    PerceptionEntry,
    PerceptionBuilder,
    DefaultPerceptionBuilder,
    ManifestPath,
};
