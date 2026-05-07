use biosphere_foundation::BasicWorld;
use biosphere_core::Environment;

fn main() {
    println!("=== First Observation Example ===\n");
    
    let mut world = BasicWorld::new();
    
    println!("Initial world state:");
    let model = world.conditions().snapshot();
    println!("Signals: {}\n", model.signals.len());
    
    for step in 1..=5 {
        println!("Step {}:", step);
        world.step();
        
        let model = world.conditions().snapshot();
        println!("Signals: {}\n", model.signals.len());
    }
    
    println!("=== First Observation Complete ===");
}
