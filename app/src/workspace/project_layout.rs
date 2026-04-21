use gui::canvas::camera::Camera;
use gui::context::Context;
use gui::panel::PanelLayout as GuiPanelLayout;
use gui::renderer::Rect;

pub(crate) const PROJECT_LAYOUT_VERSION: u32 = 1;

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
    pub(crate) collapsed: bool,
}

pub(crate) fn export_project_layout(gui: &Context, camera: &Camera) -> ProjectLayout {
    ProjectLayout {
        version: PROJECT_LAYOUT_VERSION,
        camera: CameraLayout::from_camera(camera),
        panels: gui
            .export_panel_layouts()
            .into_iter()
            .map(PanelLayout::from_gui)
            .collect(),
        canvas_nodes: Vec::new(),
    }
}

pub(crate) fn import_project_layout(gui: &mut Context, camera: &mut Camera, layout: ProjectLayout) {
    layout.camera.apply_to_camera(camera);
    let panel_layouts: Vec<GuiPanelLayout> = layout
        .panels
        .into_iter()
        .map(PanelLayout::into_gui)
        .collect();
    gui.import_panel_layouts(&panel_layouts);
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
}
