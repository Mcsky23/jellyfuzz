pub mod v8;
pub mod profile;

pub fn get_profile(name: &str) -> Option<impl profile::JsEngineProfile> {
    match name {
        "v8" => Some(v8::V8Profile),
        _ => None,
    }
}