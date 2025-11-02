// enum JsEngineProfile {
//     V8(V8Profile)
// }

pub trait JsEngineProfile {
    fn get_path(&self) -> String;
    fn get_args(&self) -> Vec<String>;
    fn fuzz_worker_job_queue_size(&self) -> usize;
    fn get_timeout(&self) -> u64;
    fn get_jobs_per_process(&self) -> usize;
}
