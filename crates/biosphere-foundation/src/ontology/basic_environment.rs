use biosphere_core::{Environment, TemporalEnvironment, WorldState, ConditionSnapshot};
use crate::world::world_clock::WorldClock;
use crate::conditions::sensed_conditions::SensedConditions;
use crate::temporal::Tick;

#[derive(Debug)]
pub struct BasicEnvironment {
    clock: WorldClock,
    conditions: SensedConditions,
}

impl BasicEnvironment {
    pub fn new() -> Self {
        let clock = WorldClock::new();
        let snapshot = ConditionSnapshot { signals: Vec::new() };
        let conditions = SensedConditions::new(snapshot);
        
        Self {
            clock,
            conditions,
        }
    }
}

impl Environment for BasicEnvironment {
    type State = BasicEnvironmentState;
    type Conditions = SensedConditions;
    
    fn step(&mut self) {
        let _new_tick = self.clock.advance().unwrap();
        let snapshot = ConditionSnapshot { signals: Vec::new() };
        self.conditions = SensedConditions::new(snapshot);
    }
    
    fn conditions(&self) -> &Self::Conditions {
        &self.conditions
    }
}

impl TemporalEnvironment for BasicEnvironment {
    fn advance(&mut self) {
        self.clock.advance().expect("Time advance should satisfy world axioms");
        let snapshot = ConditionSnapshot { signals: Vec::new() };
        self.conditions = SensedConditions::new(snapshot);
    }
}

impl BasicEnvironment {
    pub fn current_tick(&self) -> Tick {
        Tick::new(self.clock.current_tick())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasicEnvironmentState {
    tick: u64,
}

impl BasicEnvironmentState {
    pub fn new(tick: u64) -> Self {
        Self { tick }
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }
}

impl WorldState for BasicEnvironmentState {}
