/// Resize 手势命中的边或角。
///
/// 8 个方向覆盖矩形的所有可 resize 位置。TopLeft / TopRight / BottomLeft /
/// BottomRight 是角（两个方向同时 resize），Top / Bottom / Left / Right 是
/// 单方向边。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
