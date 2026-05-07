use crate::hardware::{HardwareDetector, HardwareInfo, ConfigRecommender, TrainingRecommendation};
use crate::types::TaskType;

pub struct HardwareService {
    detector: HardwareDetector,
    recommender: ConfigRecommender,
}

impl HardwareService {
    pub fn new() -> Self {
        Self {
            detector: HardwareDetector::new(),
            recommender: ConfigRecommender::new(),
        }
    }

    pub fn detect(&self) -> crate::core::Result<HardwareInfo> {
        self.detector.detect()
    }

    pub fn recommend(&self, hardware: &HardwareInfo, task: TaskType, data_size: usize) -> TrainingRecommendation {
        self.recommender.recommend(hardware, task, data_size)
    }
}

impl std::fmt::Debug for HardwareService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HardwareService").finish()
    }
}
