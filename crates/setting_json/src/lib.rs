#[cfg(all(feature = "infer-python", feature = "infer-rust"))]
compile_error!("infer-python and infer-rust cannot be enabled at the same time");

macro_rules! feature_repetition {
    ($cfg:ident = $feat:literal, $($item:item),*, $(,)?) => {
        $(
            #[cfg($cfg = $feat)]
            $item
        )*
    };
}

feature_repetition!(
    feature = "infer-python",
    mod infer_python;,
    pub use infer_python::SettingJson;,
    pub use infer_python::BotLang;,
    pub use infer_python::InferLang;,
);

feature_repetition!(
    feature = "infer-rust",
    mod infer_rust;,
    pub use infer_rust::SettingJson;,
    pub use infer_rust::BotLang;,
);
