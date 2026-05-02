use gui::canvas::camera::Camera;
use gui::canvas::CanvasNodeLayout as GuiCanvasNodeLayout;
use gui::context::Context;
use gui::panel::PanelLayout as GuiPanelLayout;
use gui::renderer::Rect;

pub(crate) const PROJECT_LAYOUT_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectLayout {
    pub(crate) version: u32,
    pub(crate) camera: CameraLayout,
    pub(crate) panels: Vec<PanelLayout>,
    pub(crate) canvas_nodes: Vec<CanvasNodeLayout>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CameraLayout {
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) zoom: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PanelLayout {
    pub(crate) id: String,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) visible: bool,
    pub(crate) z_index: i32,
    pub(crate) collapsed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CanvasNodeLayout {
    pub(crate) owner_id: String,
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) w: f32,
    pub(crate) h: f32,
    pub(crate) z_index: i32,
    pub(crate) collapsed: bool,
    pub(crate) user_min_height: Option<f32>,
}

pub(crate) fn export_project_layout(gui: &Context, camera: &Camera) -> ProjectLayout {
    ProjectLayout {
        version: PROJECT_LAYOUT_VERSION,
        camera: CameraLayout::from_camera(camera),
        panels: gui
            .panel()
            .export_layouts()
            .into_iter()
            .map(PanelLayout::from_gui)
            .collect(),
        canvas_nodes: gui
            .canvas()
            .export_node_layouts()
            .into_iter()
            .map(CanvasNodeLayout::from_gui)
            .collect(),
    }
}

pub(crate) fn import_project_layout(gui: &mut Context, camera: &mut Camera, layout: ProjectLayout) {
    layout.camera.apply_to_camera(camera);
    let panel_layouts: Vec<GuiPanelLayout> = layout
        .panels
        .into_iter()
        .map(PanelLayout::into_gui)
        .collect();
    gui.panel_mut().import_layouts(&panel_layouts);
    let canvas_node_layouts: Vec<GuiCanvasNodeLayout> = layout
        .canvas_nodes
        .into_iter()
        .map(CanvasNodeLayout::into_gui)
        .collect();
    gui.canvas_mut().import_node_layouts(&canvas_node_layouts);
}

impl CameraLayout {
    fn from_camera(camera: &Camera) -> Self {
        Self {
            x: camera.x,
            y: camera.y,
            zoom: camera.zoom,
        }
    }

    fn apply_to_camera(self, camera: &mut Camera) {
        camera.x = self.x;
        camera.y = self.y;
        camera.zoom = self.zoom;
    }
}

impl PanelLayout {
    fn from_gui(layout: GuiPanelLayout) -> Self {
        Self {
            id: layout.id,
            x: layout.rect.x,
            y: layout.rect.y,
            w: layout.rect.w,
            h: layout.rect.h,
            visible: layout.visible,
            z_index: layout.z_index,
            collapsed: layout.collapsed,
        }
    }

    fn into_gui(self) -> GuiPanelLayout {
        GuiPanelLayout {
            id: self.id,
            rect: Rect {
                x: self.x,
                y: self.y,
                w: self.w,
                h: self.h,
            },
            visible: self.visible,
            z_index: self.z_index,
            collapsed: self.collapsed,
        }
    }
}

impl CanvasNodeLayout {
    fn from_gui(layout: GuiCanvasNodeLayout) -> Self {
        Self {
            owner_id: layout.owner_id,
            x: layout.rect.x,
            y: layout.rect.y,
            w: layout.rect.w,
            h: layout.rect.h,
            z_index: layout.z_index,
            collapsed: layout.collapsed,
            user_min_height: layout.user_min_height,
        }
    }

    fn into_gui(self) -> GuiCanvasNodeLayout {
        GuiCanvasNodeLayout {
            owner_id: self.owner_id,
            rect: Rect {
                x: self.x,
                y: self.y,
                w: self.w,
                h: self.h,
            },
            z_index: self.z_index,
            collapsed: self.collapsed,
            user_min_height: self.user_min_height,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_layout_roundtrips_camera() {
        let gui = Context::new();
        let mut camera = Camera::new();
        camera.x = 120.0;
        camera.y = -40.0;
        camera.zoom = 1.75;

        let layout = export_project_layout(&gui, &camera);
        let mut imported_gui = Context::new();
        let mut imported_camera = Camera::new();

        import_project_layout(&mut imported_gui, &mut imported_camera, layout);

        assert_eq!(imported_camera.x, 120.0);
        assert_eq!(imported_camera.y, -40.0);
        assert_eq!(imported_camera.zoom, 1.75);
    }

    #[test]
    fn panel_layout_conversion_does_not_include_transient_runtime() {
        let gui_layout = GuiPanelLayout {
            id: "engine".to_string(),
            rect: Rect {
                x: 10.0,
                y: 20.0,
                w: 300.0,
                h: 180.0,
            },
            visible: true,
            z_index: 4,
            collapsed: false,
        };

        let layout = PanelLayout::from_gui(gui_layout);

        assert_eq!(
            layout,
            PanelLayout {
                id: "engine".to_string(),
                x: 10.0,
                y: 20.0,
                w: 300.0,
                h: 180.0,
                visible: true,
                z_index: 4,
                collapsed: false,
            }
        );
    }

    #[test]
    fn canvas_node_layout_conversion_does_not_include_transient_runtime() {
        let gui_layout = GuiCanvasNodeLayout {
            owner_id: "engine_node::1".to_string(),
            rect: Rect {
                x: 40.0,
                y: 80.0,
                w: 220.0,
                h: 96.0,
            },
            z_index: 9,
            collapsed: true,
            user_min_height: Some(180.0),
        };

        let layout = CanvasNodeLayout::from_gui(gui_layout);

        assert_eq!(
            layout,
            CanvasNodeLayout {
                owner_id: "engine_node::1".to_string(),
                x: 40.0,
                y: 80.0,
                w: 220.0,
                h: 96.0,
                z_index: 9,
                collapsed: true,
                user_min_height: Some(180.0),
            }
        );
    }
}
