pub mod profile;
pub mod v8;

pub fn get_profile(name: &str) -> Option<impl profile::JsEngineProfile + Clone> {
    match name {
        "v8" => Some(v8::V8Profile),
        _ => None,
    }
}
