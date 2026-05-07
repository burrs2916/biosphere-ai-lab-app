use biosphere_foundation::{
    recovery::{
        RecoveryContext, RecoveryStrategy, WorldRecovery, RecoveryResult
    },
    error::FoundationError,
    temporal::Tick,
};

struct WorldRecoveryContext {
    current_tick: u64,
    history: Vec<u64>,
}

impl WorldRecoveryContext {
    fn new() -> Self {
        Self {
            current_tick: 0,
            history: vec![0],
        }
    }

    fn advance(&mut self) {
        self.current_tick += 1;
        self.history.push(self.current_tick);
    }
}

impl RecoveryContext for WorldRecoveryContext {
    fn current_tick(&self) -> Tick {
        Tick::new(self.current_tick)
    }

    fn can_rollback(&self) -> bool {
        self.current_tick > 0
    }

    fn rollback_to(&mut self, tick: Tick) -> Result<(), FoundationError> {
        let target = tick.value();
        if target >= self.current_tick {
            return Err(FoundationError::temporal_violation(
                format!("Cannot rollback to future tick: {} >= {}", target, self.current_tick)
            ));
        }
        
        if !self.history.contains(&target) {
            return Err(FoundationError::temporal_violation(
                format!("Tick {} not in history", target)
            ));
        }
        
        self.current_tick = target;
        Ok(())
    }
}

fn main() {
    println!("=== Recovery v2 Demo ===\n");
    
    let mut world = WorldRecoveryContext::new();
    
    println!("Initial state:");
    println!("  Current tick: {}", world.current_tick());
    println!("  Can rollback: {}\n", world.can_rollback());
    
    world.advance();
    world.advance();
    world.advance();
    
    println!("After advancing 3 ticks:");
    println!("  Current tick: {}", world.current_tick());
    println!("  Can rollback: {}\n", world.can_rollback());
    
    let strategy = RecoveryStrategy::Retry { max_attempts: 3 };
    
    println!("=== Test 1: Successful operation ===");
    let result = strategy.recover(&mut world, || {
        Ok::<i32, FoundationError>(42)
    });
    println!("Result: {:?}", result);
    println!("Is ok: {}", result.is_ok());
    println!("Is failed: {}", result.is_failed());
    println!("Value: {:?}\n", result.value());
    
    println!("=== Test 2: Failed operation ===");
    let result = strategy.recover(&mut world, || {
        Err::<i32, FoundationError>(FoundationError::temporal_violation("Test error"))
    });
    println!("Result: {:?}", result);
    println!("Is ok: {}", result.is_ok());
    println!("Is failed: {}", result.is_failed());
    println!("Value: {:?}\n", result.value());
    
    println!("=== Test 3: World recovery ===");
    let world_recovery = WorldRecovery::RollbackTo(Tick::new(1));
    println!("World recovery: {:?}", world_recovery);
    
    match world.rollback_to(Tick::new(1)) {
        Ok(_) => println!("Rollback successful"),
        Err(e) => println!("Rollback failed: {:?}", e),
    }
    
    println!("  Current tick: {}", world.current_tick());
    println!("  Can rollback: {}\n", world.can_rollback());
    
    println!("=== Test 4: RecoveryResult variants ===");
    
    let ok_result: RecoveryResult<i32> = RecoveryResult::Ok(42);
    println!("Ok result: {:?}", ok_result);
    println!("  Is ok: {}", ok_result.is_ok());
    println!("  Is operation recovered: {}", ok_result.is_operation_recovered());
    println!("  Is world recovered: {}", ok_result.is_world_recovered());
    println!("  Value: {:?}\n", ok_result.value());
    
    let op_recovered: RecoveryResult<i32> = RecoveryResult::OperationRecovered {
        value: 100,
        strategy: RecoveryStrategy::Default,
    };
    println!("Operation recovered result: {:?}", op_recovered);
    println!("  Is ok: {}", op_recovered.is_ok());
    println!("  Is operation recovered: {}", op_recovered.is_operation_recovered());
    println!("  Is world recovered: {}", op_recovered.is_world_recovered());
    println!("  Value: {:?}\n", op_recovered.value());
    
    let world_recovered: RecoveryResult<i32> = RecoveryResult::WorldRecovered {
        recovery: WorldRecovery::BranchFrom(Tick::new(5)),
    };
    println!("World recovered result: {:?}", world_recovered);
    println!("  Is ok: {}", world_recovered.is_ok());
    println!("  Is operation recovered: {}", world_recovered.is_operation_recovered());
    println!("  Is world recovered: {}", world_recovered.is_world_recovered());
    println!("  Value: {:?}", world_recovered.value());
    println!("  World recovery: {:?}\n", world_recovered.world_recovery());
    
    let failed: RecoveryResult<i32> = RecoveryResult::Failed {
        error: FoundationError::temporal_violation("Test error"),
        strategy: RecoveryStrategy::Terminate,
    };
    println!("Failed result: {:?}", failed);
    println!("  Is ok: {}", failed.is_ok());
    println!("  Is failed: {}", failed.is_failed());
    println!("  Value: {:?}\n", failed.value());
    
    println!("=== Test 5: Convert to FoundationResult ===");
    
    let ok_result: RecoveryResult<i32> = RecoveryResult::Ok(42);
    let foundation_result = ok_result.to_foundation_result();
    println!("Ok -> FoundationResult: {:?}", foundation_result);
    
    let op_recovered: RecoveryResult<i32> = RecoveryResult::OperationRecovered {
        value: 100,
        strategy: RecoveryStrategy::Default,
    };
    let foundation_result = op_recovered.to_foundation_result();
    println!("OperationRecovered -> FoundationResult: {:?}", foundation_result);
    
    let world_recovered: RecoveryResult<i32> = RecoveryResult::WorldRecovered {
        recovery: WorldRecovery::BranchFrom(Tick::new(5)),
    };
    let foundation_result = world_recovered.to_foundation_result();
    println!("WorldRecovered -> FoundationResult: {:?}", foundation_result);
    
    let failed: RecoveryResult<i32> = RecoveryResult::Failed {
        error: FoundationError::temporal_violation("Test error"),
        strategy: RecoveryStrategy::Terminate,
    };
    let foundation_result = failed.to_foundation_result();
    println!("Failed -> FoundationResult: {:?}", foundation_result);
}
