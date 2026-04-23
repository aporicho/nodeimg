pub(crate) struct BuiltinIconAsset {
    pub(crate) id: &'static str,
    pub(crate) svg: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/icon_assets_generated.rs"));

pub mod names {
    include!(concat!(env!("OUT_DIR"), "/icon_names_generated.rs"));
}

pub(crate) fn assets() -> &'static [BuiltinIconAsset] {
    BUILTIN_ICON_ASSETS
}
