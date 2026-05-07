use biosphere_core::Embodiment;
use crate::ontology::{
    BasicExistence,
    BasicPerception,
    BasicRepresentation,
};

pub struct BasicEmbodiment {
    existence: BasicExistence,
    perception: BasicPerception,
    representation: BasicRepresentation,
}

impl BasicEmbodiment {
    pub fn new(existence: BasicExistence, perception: BasicPerception, representation: BasicRepresentation) -> Self {
        Self {
            existence,
            perception,
            representation,
        }
    }
}

impl Embodiment for BasicEmbodiment {
    type Existence = BasicExistence;
    type PerceptionImpl = BasicPerception;
    type RepresentationImpl = BasicRepresentation;
    
    fn existence(&self) -> &Self::Existence {
        &self.existence
    }
    
    fn perception(&self) -> &Self::PerceptionImpl {
        &self.perception
    }
    
    fn representation(&self) -> &Self::RepresentationImpl {
        &self.representation
    }
}
