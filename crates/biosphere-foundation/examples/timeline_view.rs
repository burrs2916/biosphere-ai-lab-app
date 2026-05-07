use biosphere_foundation::{BasicWorld, TimelineView};
use biosphere_core::Environment;

fn main() {
    println!("=== Timeline View Example ===\n");
    
    let mut world = BasicWorld::new();
    
    println!("Initial state (no history):");
    let timeline = TimelineView::unlimited();
    let model = timeline.render(&world, 0, u64::MAX);
    println!("Ticks: {:?}\n", model.ticks());
    
    println!("Advancing world 5 steps...");
    for step in 1..=5 {
        world.step();
        println!("Step {} completed", step);
    }
    
    println!("\nTimeline view (unlimited):");
    let timeline = TimelineView::unlimited();
    let model = timeline.render(&world, 0, u64::MAX);
    println!("Ticks: {:?}\n", model.ticks());
    
    println!("Timeline view (limited to last 3):");
    let timeline = TimelineView::limited(3);
    let model = timeline.render(&world, 0, u64::MAX);
    println!("Ticks: {:?}\n", model.ticks());
    
    println!("=== Timeline View Complete ===");
}
