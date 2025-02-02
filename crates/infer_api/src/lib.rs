#[cfg(all(feature = "infer-python", feature = "infer-rust"))]
compile_error!("infer-python and infer-rust Feature cannot be enabled at the same time");

#[cfg(all(not(feature = "infer-python"), not(feature = "infer-rust")))]
compile_error!("Feature either infer-python or infer-rust must be enabled");

macro_rules! feature_repetition {
    ($cfg:ident = $feat:literal, $($item:item),*, $(,)?) => {
        $(
            #[cfg($cfg = $feat)]
            $item
        )*
    };
}

mod client;
mod modelinfo;

#[cfg(feature = "infer-python")]
pub use client::Sbv2PythonClient;

#[cfg(feature = "infer-rust")]
pub use client::Sbv2RustClient;

pub use modelinfo::{ModelInfo, Sbv2InferParam, Sbv2ModelInfo, ValidModel, SBV2_MODELINFO};
