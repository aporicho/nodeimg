use crate::panel_tree::{reconcile, layout, paint, hit_test, Desc, PanelTree};
use crate::renderer::{Color, Rect, RectStyle, Renderer};
use crate::shell::{App, AppContext, AppEvent};

const PANEL_RECT: Rect = Rect { x: 50.0, y: 50.0, w: 300.0, h: 200.0 };
const PADDING: f32 = 16.0;

const COLOR_ACTIVE: Color = Color { r: 0.2, g: 0.5, b: 0.9, a: 1.0 };
const COLOR_INACTIVE: Color = Color { r: 0.3, g: 0.3, b: 0.35, a: 1.0 };

pub struct DemoApp {
    tree: PanelTree,
    active_button: Option<&'static str>,
}

impl DemoApp {
    fn view(&self) -> Desc {
        let active = self.active_button;
        Desc::Column {
            children: vec![
                Desc::Button {
                    id: "btn_a",
                    label: "Button A",
                    color: if active == Some("btn_a") { COLOR_ACTIVE } else { COLOR_INACTIVE },
                },
                Desc::Button {
                    id: "btn_b",
                    label: "Button B",
                    color: if active == Some("btn_b") { COLOR_ACTIVE } else { COLOR_INACTIVE },
                },
            ],
        }
    }
}

impl App for DemoApp {
    fn init(_ctx: &mut AppContext) -> Self {
        Self {
            tree: PanelTree::new(),
            active_button: None,
        }
    }

    fn event(&mut self, event: AppEvent, _ctx: &mut AppContext) {
        if let AppEvent::MousePress { x, y } = event {
            if let Some(root) = self.tree.root() {
                if let Some(id) = hit_test(&self.tree, root, x, y) {
                    self.active_button = Some(id);
                }
            }
        }
    }

    fn update(&mut self, _ctx: &mut AppContext) {
        let desc = self.view();
        reconcile(&mut self.tree, &desc);

        let content_rect = Rect {
            x: PANEL_RECT.x + PADDING,
            y: PANEL_RECT.y + PADDING,
            w: PANEL_RECT.w - PADDING * 2.0,
            h: PANEL_RECT.h - PADDING * 2.0,
        };
        if let Some(root) = self.tree.root() {
            layout(&mut self.tree, root, content_rect);
        }
    }

    fn render(&mut self, renderer: &mut Renderer, _ctx: &AppContext) {
        // 面板背景
        renderer.draw_rect(PANEL_RECT, &RectStyle {
            color: Color { r: 0.18, g: 0.18, b: 0.2, a: 1.0 },
            border: None,
            radius: [12.0; 4],
            shadow: None,
        });

        // 面板内容
        if let Some(root) = self.tree.root() {
            paint(&self.tree, root, renderer);
        }
    }
}
