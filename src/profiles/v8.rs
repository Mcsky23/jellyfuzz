use crate::profiles::profile::JsEngineProfile;

#[derive(Clone)]
pub struct V8Profile;

impl JsEngineProfile for V8Profile {
    fn get_path(&self) -> String {
        "/home/mcsky/Desktop/CTF/v8_research2/v8/out/fuzzbuild/d8".to_string()
    }

    fn get_args(&self) -> Vec<String> {
        [
            "--fuzzing".to_string(),
            "--allow-natives-syntax".to_string()
        ].to_vec()
    }

    /// The size of the mpsc job queue for each FuzzWorker
    fn fuzz_worker_job_queue_size(&self) -> usize {
        1000
    }

    /// timeout in milliseconds of each script execution
    fn get_timeout(&self) -> u64 {
        500
    }

    /// number of scripts to execute before restarting the FuzzProcess
    fn get_jobs_per_process(&self) -> usize {
        400
    }

    /// number of newly discovered coverage edges required to add input to corpus
    fn get_min_new_edges_to_add_corpus(&self) -> usize {
        10
    }
}
