#[cfg(all(feature = "infer-python", feature = "infer-rust"))]
compile_error!("infer-python and infer-rust Feature cannot be enabled at the same time");

#[cfg(all(not(feature = "infer-python"), not(feature = "infer-rust")))]
compile_error!("Feature either infer-python or infer-rust must be enabled");

mod setting_json;
pub use setting_json::BotLang;
pub use setting_json::SettingJson;

#[cfg(feature = "infer-python")]
pub use setting_json::InferLang;
