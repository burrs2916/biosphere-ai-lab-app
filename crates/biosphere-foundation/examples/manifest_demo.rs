use biosphere_foundation::{
    manifest::{
        Manifest, ManifestNode, NodeKind, Value, Derivation
    },
    temporal::{
        StateSnapshot, StatePayload, Tick
    },
};

struct DebugDerivation;

impl Derivation for DebugDerivation {
    fn derive(&self, snapshot: &StateSnapshot) -> Manifest {
        let tick = snapshot.tick().value();
        
        let root = ManifestNode::group(
        Value::text("DebugView"),
        vec![
            ManifestNode::scalar(Value::text(format!("Tick: {}", tick))),
            ManifestNode::scalar(Value::text(format!("Payload: {:?}", snapshot.payload()))),
        ],
    );
        
        Manifest::new(tick, root)
    }
}

fn main() {
    println!("=== Manifest Demo ===\n");
    
    let payload = StatePayload::new("Hello, Manifest!");
    let tick = Tick::new(42);
    let snapshot = StateSnapshot::new(tick, payload);
    
    let derivation = DebugDerivation;
    let manifest = derivation.derive(&snapshot);
    
    println!("Generated Manifest:\n");
    println!("{}", manifest);
    
    println!("\n=== Value Examples ===\n");
    
    let number = Value::number(3.14159);
    let text = Value::text("Hello");
    let boolean = Value::boolean(true);
    let tuple = Value::tuple(vec![Value::number(1.0), Value::text("two")]);
    let map = Value::map(vec![
        ("key1".to_string(), Value::text("value1")),
        ("key2".to_string(), Value::number(42.0)),
    ]);
    let opaque = Value::opaque("BinaryData");
    
    println!("Number: {}", number);
    println!("Text: {}", text);
    println!("Boolean: {}", boolean);
    println!("Tuple: {}", tuple);
    println!("Map: {}", map);
    println!("Opaque: {}", opaque);
    
    println!("\n=== ManifestNode Examples ===\n");
    
    let scalar_node = ManifestNode::scalar(Value::number(42.0));
    println!("Scalar Node:\n{}", scalar_node);
    
    let group_node = ManifestNode::group(
        Value::text("Group"),
        vec![
            ManifestNode::scalar(Value::text("Item 1")),
            ManifestNode::scalar(Value::text("Item 2")),
        ],
    );
    println!("Group Node:\n{}", group_node);
    
    let sequence_node = ManifestNode::sequence(
        Value::text("Sequence"),
        vec![
            ManifestNode::scalar(Value::number(1.0)),
            ManifestNode::scalar(Value::number(2.0)),
            ManifestNode::scalar(Value::number(3.0)),
        ],
    );
    println!("Sequence Node:\n{}", sequence_node);
    
    println!("\n=== Complex Manifest Example ===\n");
    
    let complex_root = ManifestNode::group(
        Value::text("WorldState"),
        vec![
            ManifestNode::group(
                Value::text("Entities"),
                vec![
                    ManifestNode::scalar(Value::text("Entity #1")),
                    ManifestNode::scalar(Value::text("Entity #2")),
                ],
            ),
            ManifestNode::group(
                Value::text("Environment"),
                vec![
                    ManifestNode::scalar(Value::text("Temperature: 25.0")),
                    ManifestNode::scalar(Value::text("Humidity: 60.0")),
                ],
            ),
        ],
    );
    
    let complex_manifest = Manifest::new(100, complex_root);
    println!("{}", complex_manifest);
}
