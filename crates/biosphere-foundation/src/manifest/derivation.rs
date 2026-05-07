use crate::manifest::Manifest;
use crate::temporal::state::snapshot::StateSnapshot;

pub trait Derivation {
    fn derive(&self, snapshot: &StateSnapshot) -> Manifest;
}
