#[cfg(all(feature = "infer-python", feature = "infer-rust"))]
compile_error!("infer-python and infer-rust Feature cannot be enabled at the same time");

#[cfg(all(not(feature = "infer-python"), not(feature = "infer-rust")))]
compile_error!("Feature either infer-python or infer-rust must be enabled");
