mod cache;
mod convert;
mod model;
mod parse;
mod resolve;

#[cfg(test)]
mod tests;

pub(crate) use cache::SvgVectorCache;
pub(crate) use resolve::resolve_svg_icon_paths;
