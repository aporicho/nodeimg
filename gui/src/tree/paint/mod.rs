mod context;
mod fragment;
mod icon_svg;
mod leaf;
mod node;
mod traversal;

#[cfg(test)]
mod animation_tests;
#[cfg(test)]
mod fragment_tests;
#[cfg(test)]
mod icon_svg_tests;
#[cfg(test)]
mod leaf_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod traversal_tests;

pub(crate) use context::PaintCx;
pub(crate) use fragment::build_paint_fragment;
#[cfg(test)]
pub(crate) use node::paint_to_target;
#[cfg(test)]
pub(crate) use traversal::PaintTraversal;
