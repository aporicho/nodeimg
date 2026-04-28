mod backend_command;
mod clip;
mod command;
mod composition;
mod display_list;
mod fragment;
pub mod image;
mod layer;
pub mod path;
mod recorder;
mod recording;
mod resource;
mod state;
pub mod style;
mod svg;
mod target;

pub use backend_command::BackendPaintCommand;
pub use clip::{ClipId, ClipShape, ResolvedClip};
pub use command::{
    CirclePaint, GridPaint, ImagePaint, PaintCommand, PathPaint, RectPaint, ShadowPaint,
    SvgRasterPaint, TextPaint,
};
pub use composition::{compose_fragments, PaintCompositionStats};
pub use display_list::{DisplayList, DisplayListBuilder, PaintBuildError, ResolvedPaintCommand};
pub use fragment::{FragmentChildRef, PaintFlushStats, PaintFragment};
pub use image::{
    resolve_image_draw, ImageFilter, ImageFit, ImageOpacity, ImageSourceRect, ImageStyle,
    ResolvedImageDraw, TextureSize,
};
pub use layer::LayerPaint;
pub use path::{PathCommand, PathData, PathRequest, PathStyle};
pub use recorder::BoundaryPaintRecorder;
pub use recording::RecordingPaintTarget;
pub use resource::TextureHandle;
pub use style::{
    Border, Color, Fill, FillRule, LineCap, LineJoin, RectStyle, Shadow, Stroke, TextFamily,
    TextStyle, TextWeight,
};
pub use svg::{SvgFit, SvgPaint, SvgPaintOverride, SvgSourceKey, SvgStrokeWidth, SvgStyle};
pub use target::PaintTarget;
