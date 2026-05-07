pub mod entity;
pub mod filter;
pub mod manager;
pub mod query;
pub mod existence;
pub mod identity;
pub mod state;
pub mod query_model;

pub use entity::{Entity, EntityKind};
pub use filter::EntityFilter;
pub use manager::EntityManager;
pub use query::EntityQuery;
pub use existence::EntityExistence;
pub use identity::EntityIdentitySpace;
pub use state::WorldEntityState;
pub use query_model::{EntityQuery as EntityQueryModel, TimeRange, QueryContext};

impl EntityQuery for EntityManager {
    fn get(&self, id: biosphere_core::EntityId) -> Option<&Entity> {
        manager::EntityManager::get(self, id)
    }

    fn query(&self, filter: EntityFilter) -> Vec<biosphere_core::EntityId> {
        manager::EntityManager::query(self, filter)
    }

    fn all_ids(&self) -> Vec<biosphere_core::EntityId> {
        manager::EntityManager::all_ids(self)
    }

    fn len(&self) -> usize {
        manager::EntityManager::len(self)
    }

    fn is_empty(&self) -> bool {
        manager::EntityManager::is_empty(self)
    }
}
