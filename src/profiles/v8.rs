use crate::profiles::profile::JsEngineProfile;

pub struct V8Profile;

impl JsEngineProfile for V8Profile {
    fn get_path(&self) -> String {
        "/home/mcsky/Desktop/CTF/v8_research2/v8/out/fuzzbuild/d8".to_string()
    }

    fn get_args(&self) -> Vec<String> {
        ["--fuzzing".to_string()].to_vec()
    }

    /// The size of the mpsc job queue for each FuzzWorker
    fn fuzz_worker_job_queue_size(&self) -> usize {
        5000
    }

    /// timeout in milliseconds of each script execution
    fn get_timeout(&self) -> u64 {
        1000
    }

    /// number of scripts to execute before restarting the FuzzProcess
    fn get_jobs_per_process(&self) -> usize {
        400
    }
}
