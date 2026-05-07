use burn::backend::Wgpu;

pub type AIBackend = Wgpu;

pub trait AudioFeatureExtractor: Send + Sync {
    type Output;
    
    fn extract(&self, audio_data: &[f32]) -> Self::Output;
}

pub trait StyleGenerator: Send + Sync {
    type Style;
    type Config;
    
    fn generate(&self, config: &Self::Config) -> Self::Style;
}

pub trait BehaviorPredictor: Send + Sync {
    type Behavior;
    type Input;
    
    fn predict(&self, input: &Self::Input) -> Self::Behavior;
}
