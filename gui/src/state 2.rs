<<<<<<< HEAD
use std::collections::HashSet;
use iced::widget::image::Handle;
use types::NodeId;

#[derive(Debug, Clone)]
pub struct PanelState {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelId {
    Preview,
    Toolbar,
}

#[derive(Debug, Clone)]
#[derive(Default)]
pub struct UIState {
    pub selection: HashSet<NodeId>,
    pub preview_handle: Option<Handle>,
    pub is_running: bool,
}

=======
/// GUI 全局 UI 状态。
/// 交互控制器维护，预览面板和状态栏读取渲染。
/// 画布直读引擎（此分支用 mock）图数据，不经 UIState。
#[derive(Debug, Default)]
pub struct UIState {
    // TODO: selection, execution progress, preview handle, panel visibility
}
>>>>>>> 08a5b55 (feat: iced 0.14 无限画布 + 触控板捏合缩放 patch)
