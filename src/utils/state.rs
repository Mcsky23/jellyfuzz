/// Represents the state of the fuzzer process
pub struct FuzzerState {
    pub total_jobs_executed: usize,
    pub total_crashes_found: usize,
    pub total_time_spent: u128, // in milliseconds
    pub success_rate: f64,
}

/// Configuration settings for the fuzzer
pub struct FuzzerSettings {
    // True if the initial corpus has been fully processed
    pub initial_corpus_processed: bool,
}
