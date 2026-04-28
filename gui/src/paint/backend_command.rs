use super::{GridPaint, PaintCommand};

#[derive(Debug, Clone, PartialEq)]
pub enum BackendPaintCommand {
    Paint(PaintCommand),
    Grid(GridPaint),
}
