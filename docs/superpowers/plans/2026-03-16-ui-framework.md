# UI Framework Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the UI layer to a component-based architecture with dual themes (dark/light), fix node visual defects, and polish widget styles.

**Architecture:** Theme trait with two implementations (DarkTheme, LightTheme) stored as `Arc<dyn Theme>` shared between App and NodeViewer. UI split into PreviewPanel and NodeCanvas components. Custom StyledSlider widget for polished node parameters.

**Tech Stack:** Rust, eframe/egui 0.33, egui-snarl 0.9 (sandwich layout)

**Spec:** `docs/superpowers/specs/2026-03-16-ui-framework-design.md`

---

## Chunk 1: Theme Foundation

### Task 1: Create Theme trait and design tokens

**Files:**
- Create: `src/theme/mod.rs`
- Create: `src/theme/tokens.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create `src/theme/tokens.rs`**

```rust
//! Design tokens shared by all themes. Layout constants only — no colors.

use eframe::egui::{self, vec2, Margin, Vec2};

// Spacing
pub const ITEM_SPACING: Vec2 = vec2(6.0, 4.0);
pub const BUTTON_PADDING: Vec2 = vec2(8.0, 4.0);
pub const NODE_HEADER_PADDING: Margin = Margin::symmetric(12, 6);
pub const NODE_BODY_PADDING: Margin = Margin::same(10);
pub const PARAM_LABEL_WIDTH: f32 = 80.0;

// Corner radius (u8 because CornerRadius::same() takes u8)
pub const CORNER_RADIUS: u8 = 6;
pub const NODE_CORNER_RADIUS: u8 = 10;

// Font sizes
pub const FONT_SIZE_SMALL: f32 = 11.0;
pub const FONT_SIZE_NORMAL: f32 = 13.0;

// Node constraints
pub const NODE_MAX_WIDTH: f32 = 280.0;
pub const NODE_PREVIEW_MAX_SIZE: f32 = 200.0;
```

- [ ] **Step 2: Create `src/theme/mod.rs` with Theme trait**

```rust
pub mod tokens;
pub mod dark;
pub mod light;

use eframe::egui::{self, Color32, Frame, Shadow, Stroke};

/// Defines all visual properties for a theme. Each theme implements this independently.
pub trait Theme: Send + Sync {
    // Canvas & panel
    fn canvas_bg(&self) -> Color32;
    fn panel_bg(&self) -> Color32;
    fn panel_separator(&self) -> Color32;

    // Node appearance
    fn node_body_fill(&self) -> Color32;
    fn node_body_stroke(&self) -> Stroke;
    fn node_shadow(&self) -> Shadow;
    fn node_header_text(&self) -> Color32;

    // Category colors (each theme defines its own set)
    fn category_color(&self, cat_id: &str) -> Color32;

    // Pin colors (each theme defines its own set)
    fn pin_color(&self, data_type: &str) -> Color32;

    // Text
    fn text_primary(&self) -> Color32;
    fn text_secondary(&self) -> Color32;

    // Widgets
    fn widget_bg(&self) -> Color32;
    fn widget_hover_bg(&self) -> Color32;
    fn widget_active_bg(&self) -> Color32;
    fn accent(&self) -> Color32;
    fn accent_hover(&self) -> Color32;

    // Apply full theme to egui context
    fn apply(&self, ctx: &egui::Context);

    // Node frames (used by SnarlViewer)
    fn node_header_frame(&self, cat_id: &str) -> Frame;
    fn node_body_frame(&self) -> Frame;
}
```

- [ ] **Step 3: Add theme module to `src/lib.rs`**

Add `pub mod theme;` to the existing module declarations in `src/lib.rs`.

- [ ] **Step 4: Verify it compiles**

Run: `cargo build 2>&1 | head -20`
Expected: Errors about missing `dark.rs` and `light.rs` — that's expected, we create those next.

- [ ] **Step 5: Create placeholder dark.rs and light.rs so it compiles**

Create `src/theme/dark.rs` and `src/theme/light.rs` as empty files for now:
```rust
// src/theme/dark.rs
// DarkTheme implementation — filled in Task 2
```
```rust
// src/theme/light.rs
// LightTheme implementation — filled in Task 3
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo build 2>&1 | head -5`
Expected: Compiles successfully (no references to the new trait yet).

- [ ] **Step 7: Commit**

```bash
git add src/theme/ src/lib.rs
git commit -m "feat: add Theme trait and design tokens module"
```

---

### Task 2: Implement LightTheme

**Files:**
- Modify: `src/theme/light.rs`

- [ ] **Step 1: Implement LightTheme**

Write the full `LightTheme` struct implementing `Theme` in `src/theme/light.rs`:

```rust
use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, Shadow, Stroke};
use super::Theme;
use super::tokens;

pub struct LightTheme;

impl Theme for LightTheme {
    // Canvas & panel
    fn canvas_bg(&self) -> Color32 { Color32::from_rgb(240, 241, 245) }
    fn panel_bg(&self) -> Color32 { Color32::from_rgb(250, 251, 253) }
    fn panel_separator(&self) -> Color32 { Color32::from_rgb(216, 218, 226) }

    // Node
    fn node_body_fill(&self) -> Color32 { Color32::WHITE }
    fn node_body_stroke(&self) -> Stroke { Stroke::new(1.0, Color32::from_rgb(216, 218, 226)) }
    fn node_shadow(&self) -> Shadow {
        Shadow { offset: [0, 2], blur: 8, spread: 0, color: Color32::from_black_alpha(20) }
    }
    fn node_header_text(&self) -> Color32 { Color32::WHITE }

    // Category colors — refined pastels for light bg
    fn category_color(&self, cat_id: &str) -> Color32 {
        match cat_id {
            "data"      => Color32::from_rgb(66, 133, 200),
            "generate"  => Color32::from_rgb(125, 85, 185),
            "color"     => Color32::from_rgb(205, 115, 50),
            "transform" => Color32::from_rgb(40, 160, 120),
            "filter"    => Color32::from_rgb(190, 80, 135),
            "composite" => Color32::from_rgb(180, 150, 35),
            "tool"      => Color32::from_rgb(110, 118, 138),
            _           => Color32::from_rgb(140, 145, 158),
        }
    }

    // Pin colors — good contrast on white
    fn pin_color(&self, data_type: &str) -> Color32 {
        match data_type {
            "image"   => Color32::from_rgb(56, 153, 224),
            "mask"    => Color32::from_rgb(33, 179, 130),
            "float"   => Color32::from_rgb(232, 120, 46),
            "int"     => Color32::from_rgb(0, 179, 199),
            "color"   => Color32::from_rgb(140, 92, 209),
            "boolean" => Color32::from_rgb(214, 87, 97),
            "string"  => Color32::from_rgb(120, 130, 150),
            _         => Color32::GRAY,
        }
    }

    // Text
    fn text_primary(&self) -> Color32 { Color32::from_rgb(26, 28, 36) }
    fn text_secondary(&self) -> Color32 { Color32::from_rgb(107, 112, 128) }

    // Widgets
    fn widget_bg(&self) -> Color32 { Color32::from_rgb(235, 237, 242) }
    fn widget_hover_bg(&self) -> Color32 { Color32::from_rgb(222, 226, 236) }
    fn widget_active_bg(&self) -> Color32 { Color32::from_rgb(66, 99, 235) }
    fn accent(&self) -> Color32 { Color32::from_rgb(66, 99, 235) }
    fn accent_hover(&self) -> Color32 { Color32::from_rgb(85, 120, 245) }

    fn apply(&self, ctx: &egui::Context) {
        ctx.set_visuals(egui::Visuals::light());
        ctx.style_mut(|style| {
            style.spacing.item_spacing = tokens::ITEM_SPACING;
            style.spacing.button_padding = tokens::BUTTON_PADDING;

            let cr = CornerRadius::same(tokens::CORNER_RADIUS);
            style.visuals.widgets.noninteractive.corner_radius = cr;
            style.visuals.widgets.inactive.corner_radius = cr;
            style.visuals.widgets.hovered.corner_radius = cr;
            style.visuals.widgets.active.corner_radius = cr;

            style.visuals.widgets.noninteractive.bg_fill = self.widget_bg();
            style.visuals.widgets.inactive.bg_fill = self.widget_bg();
            style.visuals.widgets.hovered.bg_fill = self.widget_hover_bg();
            style.visuals.widgets.active.bg_fill = self.widget_active_bg();

            style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, Color32::from_rgb(210, 212, 220));
            style.visuals.widgets.inactive.bg_stroke = Stroke::new(0.5, Color32::from_rgb(200, 204, 215));
            style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent_hover());
            style.visuals.widgets.active.bg_stroke = Stroke::new(1.5, self.accent());

            style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text_secondary());
            style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text_primary());
            style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.text_primary());
            style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

            style.visuals.selection.bg_fill = self.accent().gamma_multiply(0.15);
            style.visuals.selection.stroke = Stroke::new(1.0, self.accent());

            style.visuals.panel_fill = self.panel_bg();
            style.visuals.window_fill = Color32::WHITE;
            style.visuals.window_stroke = Stroke::new(1.0, self.node_body_stroke().color);
            style.visuals.window_shadow = self.node_shadow();
            style.visuals.window_corner_radius = CornerRadius::same(tokens::NODE_CORNER_RADIUS);

            style.visuals.override_text_color = Some(self.text_primary());
            style.visuals.extreme_bg_color = Color32::WHITE;
            style.visuals.faint_bg_color = Color32::from_rgb(248, 249, 252);
        });
    }

    fn node_header_frame(&self, cat_id: &str) -> Frame {
        Frame {
            inner_margin: tokens::NODE_HEADER_PADDING,
            outer_margin: Margin::ZERO,
            corner_radius: CornerRadius { nw: tokens::NODE_CORNER_RADIUS, ne: tokens::NODE_CORNER_RADIUS, sw: 0, se: 0 },
            fill: self.category_color(cat_id),
            stroke: Stroke::NONE,
            shadow: Shadow::NONE,
        }
    }

    fn node_body_frame(&self) -> Frame {
        Frame {
            inner_margin: Margin { top: 0, bottom: 10, left: 10, right: 10 },
            outer_margin: Margin::ZERO,
            corner_radius: CornerRadius::same(tokens::NODE_CORNER_RADIUS),
            fill: self.node_body_fill(),
            stroke: self.node_body_stroke(),
            shadow: self.node_shadow(),
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build 2>&1 | head -5`
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add src/theme/light.rs
git commit -m "feat: implement LightTheme (Refined Light palette)"
```

---

### Task 3: Implement DarkTheme

**Files:**
- Modify: `src/theme/dark.rs`

- [ ] **Step 1: Implement DarkTheme**

Write the full `DarkTheme` struct implementing `Theme` in `src/theme/dark.rs`. Same structure as LightTheme but with Modern Neutral palette:

```rust
use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, Shadow, Stroke};
use super::Theme;
use super::tokens;

pub struct DarkTheme;

impl Theme for DarkTheme {
    fn canvas_bg(&self) -> Color32 { Color32::from_rgb(43, 45, 48) }
    fn panel_bg(&self) -> Color32 { Color32::from_rgb(30, 31, 34) }
    fn panel_separator(&self) -> Color32 { Color32::from_rgb(60, 63, 68) }

    fn node_body_fill(&self) -> Color32 { Color32::from_rgb(57, 59, 64) }
    fn node_body_stroke(&self) -> Stroke { Stroke::new(1.0, Color32::from_rgb(78, 81, 87)) }
    fn node_shadow(&self) -> Shadow {
        Shadow { offset: [0, 2], blur: 8, spread: 0, color: Color32::from_black_alpha(40) }
    }
    fn node_header_text(&self) -> Color32 { Color32::from_rgb(220, 222, 228) }

    // Category colors — muted for dark bg
    fn category_color(&self, cat_id: &str) -> Color32 {
        match cat_id {
            "data"      => Color32::from_rgb(74, 140, 199),
            "generate"  => Color32::from_rgb(140, 105, 190),
            "color"     => Color32::from_rgb(199, 125, 74),
            "transform" => Color32::from_rgb(55, 155, 120),
            "filter"    => Color32::from_rgb(180, 90, 130),
            "composite" => Color32::from_rgb(170, 145, 45),
            "tool"      => Color32::from_rgb(100, 108, 125),
            _           => Color32::from_rgb(90, 95, 105),
        }
    }

    // Pin colors — brighter for dark bg visibility
    fn pin_color(&self, data_type: &str) -> Color32 {
        match data_type {
            "image"   => Color32::from_rgb(80, 170, 235),
            "mask"    => Color32::from_rgb(50, 195, 150),
            "float"   => Color32::from_rgb(240, 140, 65),
            "int"     => Color32::from_rgb(30, 195, 210),
            "color"   => Color32::from_rgb(160, 110, 225),
            "boolean" => Color32::from_rgb(225, 100, 110),
            "string"  => Color32::from_rgb(140, 150, 168),
            _         => Color32::GRAY,
        }
    }

    fn text_primary(&self) -> Color32 { Color32::from_rgb(176, 179, 184) }
    fn text_secondary(&self) -> Color32 { Color32::from_rgb(106, 109, 115) }

    fn widget_bg(&self) -> Color32 { Color32::from_rgb(50, 52, 56) }
    fn widget_hover_bg(&self) -> Color32 { Color32::from_rgb(60, 63, 68) }
    fn widget_active_bg(&self) -> Color32 { Color32::from_rgb(75, 110, 200) }
    fn accent(&self) -> Color32 { Color32::from_rgb(75, 110, 200) }
    fn accent_hover(&self) -> Color32 { Color32::from_rgb(90, 128, 220) }

    fn apply(&self, ctx: &egui::Context) {
        ctx.set_visuals(egui::Visuals::dark());
        ctx.style_mut(|style| {
            style.spacing.item_spacing = tokens::ITEM_SPACING;
            style.spacing.button_padding = tokens::BUTTON_PADDING;

            let cr = CornerRadius::same(tokens::CORNER_RADIUS);
            style.visuals.widgets.noninteractive.corner_radius = cr;
            style.visuals.widgets.inactive.corner_radius = cr;
            style.visuals.widgets.hovered.corner_radius = cr;
            style.visuals.widgets.active.corner_radius = cr;

            style.visuals.widgets.noninteractive.bg_fill = self.widget_bg();
            style.visuals.widgets.inactive.bg_fill = self.widget_bg();
            style.visuals.widgets.hovered.bg_fill = self.widget_hover_bg();
            style.visuals.widgets.active.bg_fill = self.widget_active_bg();

            style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, Color32::from_rgb(60, 63, 68));
            style.visuals.widgets.inactive.bg_stroke = Stroke::new(0.5, Color32::from_rgb(68, 72, 78));
            style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent_hover());
            style.visuals.widgets.active.bg_stroke = Stroke::new(1.5, self.accent());

            style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text_secondary());
            style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, self.text_primary());
            style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, self.text_primary());
            style.visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);

            style.visuals.selection.bg_fill = self.accent().gamma_multiply(0.2);
            style.visuals.selection.stroke = Stroke::new(1.0, self.accent());

            style.visuals.panel_fill = self.panel_bg();
            style.visuals.window_fill = self.node_body_fill();
            style.visuals.window_stroke = Stroke::new(1.0, self.node_body_stroke().color);
            style.visuals.window_shadow = self.node_shadow();
            style.visuals.window_corner_radius = CornerRadius::same(tokens::NODE_CORNER_RADIUS);

            style.visuals.override_text_color = Some(self.text_primary());
            style.visuals.extreme_bg_color = Color32::from_rgb(25, 26, 28);
            style.visuals.faint_bg_color = Color32::from_rgb(35, 37, 40);
        });
    }

    fn node_header_frame(&self, cat_id: &str) -> Frame {
        Frame {
            inner_margin: tokens::NODE_HEADER_PADDING,
            outer_margin: Margin::ZERO,
            corner_radius: CornerRadius { nw: tokens::NODE_CORNER_RADIUS, ne: tokens::NODE_CORNER_RADIUS, sw: 0, se: 0 },
            fill: self.category_color(cat_id),
            stroke: Stroke::NONE,
            shadow: Shadow::NONE,
        }
    }

    fn node_body_frame(&self) -> Frame {
        Frame {
            inner_margin: Margin { top: 0, bottom: 10, left: 10, right: 10 },
            outer_margin: Margin::ZERO,
            corner_radius: CornerRadius::same(tokens::NODE_CORNER_RADIUS),
            fill: self.node_body_fill(),
            stroke: self.node_body_stroke(),
            shadow: self.node_shadow(),
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build 2>&1 | head -5`

- [ ] **Step 3: Commit**

```bash
git add src/theme/dark.rs
git commit -m "feat: implement DarkTheme (Modern Neutral palette)"
```

---

## Chunk 2: UI Components and App Refactor

### Task 4: Create PreviewPanel component

**Files:**
- Create: `src/ui/mod.rs`
- Create: `src/ui/preview_panel.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create `src/ui/mod.rs`**

```rust
pub mod preview_panel;
pub mod node_canvas;
```

- [ ] **Step 2: Create `src/ui/preview_panel.rs`**

Extract preview panel logic from `src/app.rs` lines 44-77:

```rust
use eframe::egui;
use crate::theme::Theme;

pub struct PreviewPanel {
    pub show: bool,
}

impl PreviewPanel {
    pub fn new() -> Self {
        Self { show: true }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        theme: &dyn Theme,
        preview_texture: Option<&egui::TextureHandle>,
    ) {
        if !self.show {
            return;
        }

        let panel_width = ctx.content_rect().width() / 2.0;
        egui::SidePanel::left("preview_panel")
            .default_width(panel_width)
            .min_width(200.0)
            .resizable(true)
            .frame(egui::Frame {
                inner_margin: egui::Margin::same(0),
                outer_margin: egui::Margin::ZERO,
                corner_radius: egui::CornerRadius::ZERO,
                fill: theme.panel_bg(),
                stroke: egui::Stroke::NONE,
                shadow: egui::Shadow::NONE,
            })
            .show(ctx, |ui| {
                ui.add_space(4.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    if ui.add(egui::Button::new("\u{2715}").frame(false)).clicked() {
                        self.show = false;
                    }
                });

                if let Some(tex) = preview_texture {
                    let size = tex.size_vec2();
                    let available = ui.available_size();
                    let scale = (available.x / size.x)
                        .min(available.y / size.y)
                        .min(1.0);
                    ui.centered_and_justified(|ui| {
                        ui.image(egui::load::SizedTexture::new(tex.id(), size * scale));
                    });
                } else {
                    ui.centered_and_justified(|ui| {
                        ui.colored_label(theme.text_secondary(), "No preview");
                    });
                }
            });
    }
}
```

- [ ] **Step 3: Create placeholder `src/ui/node_canvas.rs`**

```rust
// NodeCanvas — filled in next step
```

- [ ] **Step 4: Add `pub mod ui;` to `src/lib.rs`**

- [ ] **Step 5: Verify it compiles**

Run: `cargo build 2>&1 | head -10`
Expected: Compiles with warnings about unused code.

---

### Task 5: Create NodeCanvas component

**Files:**
- Create: `src/ui/node_canvas.rs`

- [ ] **Step 1: Create `src/ui/node_canvas.rs`**

Extract node editor logic from `src/app.rs` lines 80-95:

```rust
use eframe::egui;
use egui::Id;
use egui_snarl::Snarl;
use egui_snarl::ui::{SnarlStyle, SnarlWidget, NodeLayout};

use crate::node::viewer::NodeViewer;
use crate::node::registry::NodeInstance;
use crate::theme::Theme;
use crate::ui::preview_panel::PreviewPanel;

pub struct NodeCanvas {
    pub style: SnarlStyle,
}

impl NodeCanvas {
    pub fn new() -> Self {
        let style = SnarlStyle {
            node_layout: Some(NodeLayout::sandwich()),
            collapsible: Some(false),
            ..SnarlStyle::new()
        };
        Self { style }
    }

    pub fn show(
        &self,
        ctx: &egui::Context,
        theme: &dyn Theme,
        snarl: &mut Snarl<NodeInstance>,
        viewer: &mut NodeViewer,
        preview_panel: &mut PreviewPanel,
    ) {
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(theme.canvas_bg()))
            .show(ctx, |ui| {
                if !preview_panel.show {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                        if ui.add(egui::Button::new("\u{25a1}").frame(false))
                            .on_hover_text("Open Preview")
                            .clicked()
                        {
                            preview_panel.show = true;
                        }
                    });
                }
                SnarlWidget::new()
                    .id(Id::new("node-image-studio"))
                    .style(self.style)
                    .show(snarl, viewer, ui);
            });
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build 2>&1 | head -10`

- [ ] **Step 3: Commit**

```bash
git add src/ui/
git commit -m "feat: add PreviewPanel and NodeCanvas UI components"
```

---

### Task 6: Refactor App to use components and theme

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Rewrite `src/app.rs`**

Replace the entire file with:

```rust
use eframe::egui;
use egui_snarl::Snarl;
use std::sync::Arc;

use crate::node::viewer::NodeViewer;
use crate::node::registry::NodeInstance;
use crate::theme::Theme;
use crate::theme::light::LightTheme;
use crate::theme::dark::DarkTheme;
use crate::ui::preview_panel::PreviewPanel;
use crate::ui::node_canvas::NodeCanvas;

pub struct App {
    snarl: Snarl<NodeInstance>,
    viewer: NodeViewer,
    preview_panel: PreviewPanel,
    node_canvas: NodeCanvas,
    theme: Arc<dyn Theme>,
    use_dark: bool,
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let theme: Arc<dyn Theme> = Arc::new(LightTheme);

        Self {
            snarl: Snarl::new(),
            viewer: NodeViewer::new(),
            preview_panel: PreviewPanel::new(),
            node_canvas: NodeCanvas::new(),
            theme,
            use_dark: false,
        }
    }

    fn toggle_theme(&mut self) {
        self.use_dark = !self.use_dark;
        self.theme = if self.use_dark {
            Arc::new(DarkTheme)
        } else {
            Arc::new(LightTheme)
        };
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Theme toggle: Ctrl+T
        if ctx.input(|i| i.key_pressed(egui::Key::T) && i.modifiers.command) {
            self.toggle_theme();
        }

        self.theme.apply(ctx);

        self.preview_panel.show(ctx, &*self.theme, self.viewer.preview_texture.as_ref());
        self.node_canvas.show(ctx, &*self.theme, &mut self.snarl, &mut self.viewer,
                              &mut self.preview_panel);
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build 2>&1 | head -10`
Expected: Should compile. Old `node::theme` is still referenced by `viewer.rs` and `types.rs` — that's fine for now.

- [ ] **Step 3: Run the app to verify nothing broke**

Run: `cargo run --release`
Expected: App opens, nodes work as before. The old theme constants are still in use via `node/theme.rs`. Visual appearance should be the same or very close.

- [ ] **Step 4: Commit**

```bash
git add src/app.rs
git commit -m "refactor: App uses PreviewPanel, NodeCanvas components and Theme"
```

---

## Chunk 3: NodeViewer Theme Integration

### Task 7: Add Arc\<dyn Theme\> to NodeViewer and migrate pin colors

**Files:**
- Modify: `src/node/viewer.rs`
- Modify: `src/node/types.rs`
- Modify: `src/app.rs`

- [ ] **Step 1: Update `NodeViewer` to accept `Arc<dyn Theme>`**

In `src/node/viewer.rs`, add the theme field:

```rust
use std::sync::Arc;
use crate::theme::Theme;
```

Add field to `NodeViewer` struct:
```rust
pub struct NodeViewer {
    pub theme: Arc<dyn Theme>,
    // ... all existing fields remain
}
```

Update `NodeViewer::new()` to accept theme:
```rust
pub fn new(theme: Arc<dyn Theme>) -> Self {
    // ... existing code, add theme field
}
```

Replace `self.pin_color()` method to use theme:
```rust
fn pin_color(&self, data_type: &DataTypeId) -> Color32 {
    self.theme.pin_color(&data_type.0)
}
```

Replace all `theme::NODE_LABEL` and `theme::TEXT_SECONDARY` references with `self.theme.text_secondary()` and `self.theme.text_primary()`.

Replace `theme::node_header_frame(&cat_id)` in `header_frame()` with `self.theme.node_header_frame(&cat_id)`.

Replace `theme::node_body_frame()` in `node_frame()` with `self.theme.node_body_frame()`.

- [ ] **Step 2: Update App to pass theme to NodeViewer**

In `src/app.rs`, change `NodeViewer::new()` call:
```rust
viewer: NodeViewer::new(Arc::clone(&theme)),
```

In `toggle_theme()`, update viewer's theme:
```rust
fn toggle_theme(&mut self) {
    self.use_dark = !self.use_dark;
    self.theme = if self.use_dark {
        Arc::new(DarkTheme)
    } else {
        Arc::new(LightTheme)
    };
    self.viewer.theme = Arc::clone(&self.theme);
    self.viewer.invalidate_all();
}
```

- [ ] **Step 3: Remove `pin_color` field from `DataTypeInfo`**

In `src/node/types.rs`:
- Remove `pin_color: [f32; 3]` from `DataTypeInfo` struct
- Remove `pub fn pin_color(&self, id: &DataTypeId) -> [f32; 3]` from `DataTypeRegistry`
- Remove `use crate::node::theme;` from `with_builtins()`
- Remove the `pin_color` field from all `DataTypeInfo` initializations in `with_builtins()`
- Update these tests that construct `DataTypeInfo` with `pin_color` — remove the field from each:
  - `test_register_and_query_type` (~line 160, 1 instance)
  - `test_compatibility_check` (~line 172, 2 instances)
  - `test_convert_value` (~line 198, 2 instances)

- [ ] **Step 4: Verify it compiles**

Run: `cargo build 2>&1 | head -20`

- [ ] **Step 5: Run tests**

Run: `cargo test 2>&1 | tail -20`
Expected: All existing tests pass.

- [ ] **Step 6: Run the app**

Run: `cargo run --release`
Expected: App works. Press Cmd+T to toggle between light and dark themes.

- [ ] **Step 7: Commit**

```bash
git add src/node/viewer.rs src/node/types.rs src/app.rs
git commit -m "feat: integrate Theme into NodeViewer, migrate pin colors to theme"
```

---

### Task 8: Delete old node/theme.rs

**Files:**
- Delete: `src/node/theme.rs`
- Modify: `src/node/mod.rs`

- [ ] **Step 1: Remove `pub mod theme;` from `src/node/mod.rs`**

- [ ] **Step 2: Delete `src/node/theme.rs`**

Run: `rm -f src/node/theme.rs`

- [ ] **Step 3: Remove any remaining `use crate::node::theme` imports**

Search all files for `node::theme` and remove/replace references. The `types.rs` reference was already handled in Task 7.

- [ ] **Step 4: Verify it compiles and tests pass**

Run: `cargo build && cargo test 2>&1 | tail -10`

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: remove old node/theme.rs, replaced by theme/ module"
```

---

## Chunk 4: Node Visual Fixes

### Task 9: Fix output pin text alignment

**Files:**
- Modify: `src/node/viewer.rs`

- [ ] **Step 1: Update `show_output` to use right-to-left layout**

In `show_output`, replace the label line with right-aligned layout:

```rust
fn show_output(&mut self, pin: &OutPin, ui: &mut Ui, snarl: &mut Snarl<NodeInstance>) -> PinInfo {
    let instance = &snarl[pin.id.node];
    if let Some(def) = self.node_registry.get(&instance.type_id) {
        if let Some(pin_def) = def.outputs.get(pin.id.output) {
            let color = self.pin_color(&pin_def.data_type);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(self.theme.text_secondary(), &pin_def.name);
            });
            return PinInfo::circle().with_fill(color);
        }
    }
    ui.colored_label(self.theme.text_secondary(), "?");
    PinInfo::circle().with_fill(Color32::GRAY)
}
```

- [ ] **Step 2: Verify visually**

Run: `cargo run --release`
Expected: Output pin "image" text is right-aligned, no longer overlapping with pin circle.

- [ ] **Step 3: Commit**

```bash
git add src/node/viewer.rs
git commit -m "fix: right-align output pin labels to prevent overlap with pin circles"
```

---

### Task 10: Fix header/body frame alignment

**Files:**
- Modify: `src/node/viewer.rs`

This is the trickiest fix. egui-snarl renders `node_frame` first as outer container, then `header_frame` on top inside it. The approach: set `node_frame` top inner margin to 0, and ensure `header_frame` fills the full width with matching top corners.

- [ ] **Step 1: Adjust node_frame and header_frame in viewer**

The `node_body_frame()` and `node_header_frame()` are already defined in the Theme implementations (Tasks 2-3) with `inner_margin.top = 0` on the body. Verify the frames are being used correctly in the viewer's `header_frame()` and `node_frame()` methods.

If the gap persists after testing, apply the fallback: override `show_header` to paint the category color rect manually:

```rust
fn show_header(
    &mut self,
    node_id: NodeId,
    _inputs: &[InPin],
    _outputs: &[OutPin],
    ui: &mut Ui,
    snarl: &mut Snarl<NodeInstance>,
) {
    let cat_id = self.node_registry.get(&snarl[node_id].type_id)
        .map(|def| def.category.0.clone())
        .unwrap_or_else(|| "tool".to_string());

    let title = self.title(&snarl[node_id]);
    let header_color = self.theme.category_color(&cat_id);

    // Paint colored background behind header text
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, CornerRadius::ZERO, header_color);

    ui.colored_label(self.theme.node_header_text(), title);
}
```

- [ ] **Step 2: Run and verify visually**

Run: `cargo run --release`
Expected: Header color fills flush to node edges, no white gap visible.

- [ ] **Step 3: Commit**

```bash
git add src/node/viewer.rs
git commit -m "fix: align node header color with outer frame edges"
```

---

### Task 11: Node max width and file picker path truncation

**Files:**
- Modify: `src/node/viewer.rs`
- Modify: `src/node/widget/file_picker.rs`

- [ ] **Step 1: Add max width constraint in `show_body`**

In `show_body`, add at the start of the vertical layout:
```rust
ui.set_max_width(crate::theme::tokens::NODE_MAX_WIDTH);
```

- [ ] **Step 2: Add tooltip to file picker display text**

In `src/node/widget/file_picker.rs`, change the display label to show tooltip with full path:

```rust
let display = if path.is_empty() {
    "Select file...".to_string()
} else {
    std::path::Path::new(path.as_str())
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("...")
        .to_string()
};
let label = ui.label(&display);
if !path.is_empty() {
    label.on_hover_text(path.as_str());
}
```

- [ ] **Step 3: Verify visually**

Run: `cargo run --release`
Expected: Load Image node is constrained to ~280px width. Long filenames show truncated with full path on hover.

- [ ] **Step 4: Commit**

```bash
git add src/node/viewer.rs src/node/widget/file_picker.rs
git commit -m "fix: constrain node max width, truncate file picker paths"
```

---

### Task 12: Parameter two-column layout

**Files:**
- Modify: `src/node/viewer.rs`

- [ ] **Step 1: Update parameter rendering in `show_body`**

In the `show_body` method, change the parameter rendering loop. Replace the current `ui.horizontal` block with a two-column layout:

```rust
for param_def in &params_def {
    ui.horizontal(|ui| {
        // Fixed-width label column
        ui.allocate_ui_with_layout(
            egui::vec2(crate::theme::tokens::PARAM_LABEL_WIDTH, ui.available_height()),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.colored_label(self.theme.text_secondary(), &param_def.name);
            },
        );

        let disabled = false;
        let constraint_type = param_def.constraint.constraint_type();
        let old_value = snarl[node_id].params.get(&param_def.name).cloned();
        if let Some(mut value) = old_value {
            let changed = self.widget_registry.render(
                param_def.widget_override.as_ref(),
                &param_def.data_type,
                &constraint_type,
                ui,
                &mut value,
                &param_def.constraint,
                &param_def.name,
                disabled,
            );
            if changed {
                snarl[node_id].params.insert(param_def.name.clone(), value);
                any_changed = true;
            }
        }
    });
}
```

- [ ] **Step 2: Verify visually**

Run: `cargo run --release`
Expected: Parameter names are left-aligned in a fixed column, widgets in the remaining space. "path" and "Select file... Browse" are cleanly separated.

- [ ] **Step 3: Commit**

```bash
git add src/node/viewer.rs
git commit -m "fix: two-column parameter layout with fixed label width"
```

---

## Chunk 5: Custom Slider Widget

### Task 13: Implement StyledSlider

**Files:**
- Modify: `src/node/widget/slider.rs`

- [ ] **Step 1: Implement custom slider rendering**

Replace `src/node/widget/slider.rs` with a custom-drawn slider:

```rust
use eframe::egui::{self, Sense, Rect, Vec2, Color32};
use crate::node::types::Value;
use crate::node::constraint::Constraint;

const TRACK_HEIGHT: f32 = 2.0;
const HANDLE_RADIUS: f32 = 5.0;
const SLIDER_WIDTH: f32 = 120.0;

/// Custom styled float slider. Returns true if value changed.
pub fn render_slider(
    ui: &mut egui::Ui,
    value: &mut Value,
    constraint: &Constraint,
    _param_name: &str,
    disabled: bool,
) -> bool {
    let Constraint::Range { min, max } = constraint else { return false; };
    let Value::Float(ref mut v) = value else { return false; };
    let min_f = *min as f32;
    let max_f = *max as f32;

    let mut changed = false;

    ui.horizontal(|ui| {
        // Draw custom slider track + handle
        let desired_size = Vec2::new(SLIDER_WIDTH, HANDLE_RADIUS * 2.0 + 4.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if !disabled && response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                *v = min_f + t * (max_f - min_f);
                changed = true;
            }
        }
        if !disabled && response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                *v = min_f + t * (max_f - min_f);
                changed = true;
            }
        }

        let painter = ui.painter();
        let t = if (max_f - min_f).abs() > f32::EPSILON {
            ((*v - min_f) / (max_f - min_f)).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let center_y = rect.center().y;

        // Track colors from egui style (theme-aware)
        let track_inactive = ui.visuals().widgets.inactive.bg_fill;
        let track_active = ui.visuals().selection.stroke.color;
        let handle_color = if disabled {
            ui.visuals().widgets.noninteractive.fg_stroke.color
        } else if response.hovered() || response.dragged() {
            track_active
        } else {
            ui.visuals().widgets.inactive.fg_stroke.color
        };

        // Inactive track (full width)
        let track_rect = Rect::from_min_max(
            egui::pos2(rect.left(), center_y - TRACK_HEIGHT / 2.0),
            egui::pos2(rect.right(), center_y + TRACK_HEIGHT / 2.0),
        );
        painter.rect_filled(track_rect, TRACK_HEIGHT / 2.0, track_inactive);

        // Active track (left portion)
        let handle_x = rect.left() + t * rect.width();
        let active_rect = Rect::from_min_max(
            egui::pos2(rect.left(), center_y - TRACK_HEIGHT / 2.0),
            egui::pos2(handle_x, center_y + TRACK_HEIGHT / 2.0),
        );
        painter.rect_filled(active_rect, TRACK_HEIGHT / 2.0, track_active);

        // Handle
        painter.circle_filled(egui::pos2(handle_x, center_y), HANDLE_RADIUS, handle_color);

        // DragValue for precise input
        let dv = ui.add_enabled(
            !disabled,
            egui::DragValue::new(v)
                .range(min_f..=max_f)
                .speed((max_f - min_f) / 200.0)
                .max_decimals(2),
        );
        if dv.changed() {
            changed = true;
        }
    });

    changed
}

/// Custom styled integer slider. Returns true if value changed.
pub fn render_int_slider(
    ui: &mut egui::Ui,
    value: &mut Value,
    constraint: &Constraint,
    _param_name: &str,
    disabled: bool,
) -> bool {
    let Constraint::Range { min, max } = constraint else { return false; };
    let Value::Int(ref mut v) = value else { return false; };
    let min_i = *min as i32;
    let max_i = *max as i32;

    let mut changed = false;
    let mut float_v = *v as f32;

    ui.horizontal(|ui| {
        let desired_size = Vec2::new(SLIDER_WIDTH, HANDLE_RADIUS * 2.0 + 4.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if !disabled && (response.dragged() || response.clicked()) {
            if let Some(pos) = response.interact_pointer_pos() {
                let t = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                float_v = (min_i as f32) + t * ((max_i - min_i) as f32);
                *v = float_v.round() as i32;
                changed = true;
            }
        }

        let painter = ui.painter();
        let range = (max_i - min_i) as f32;
        let t = if range.abs() > f32::EPSILON {
            ((*v - min_i) as f32 / range).clamp(0.0, 1.0)
        } else {
            0.5
        };
        let center_y = rect.center().y;

        let track_inactive = ui.visuals().widgets.inactive.bg_fill;
        let track_active = ui.visuals().selection.stroke.color;
        let handle_color = if disabled {
            ui.visuals().widgets.noninteractive.fg_stroke.color
        } else if response.hovered() || response.dragged() {
            track_active
        } else {
            ui.visuals().widgets.inactive.fg_stroke.color
        };

        let track_rect = Rect::from_min_max(
            egui::pos2(rect.left(), center_y - TRACK_HEIGHT / 2.0),
            egui::pos2(rect.right(), center_y + TRACK_HEIGHT / 2.0),
        );
        painter.rect_filled(track_rect, TRACK_HEIGHT / 2.0, track_inactive);

        let handle_x = rect.left() + t * rect.width();
        let active_rect = Rect::from_min_max(
            egui::pos2(rect.left(), center_y - TRACK_HEIGHT / 2.0),
            egui::pos2(handle_x, center_y + TRACK_HEIGHT / 2.0),
        );
        painter.rect_filled(active_rect, TRACK_HEIGHT / 2.0, track_active);

        painter.circle_filled(egui::pos2(handle_x, center_y), HANDLE_RADIUS, handle_color);

        let dv = ui.add_enabled(
            !disabled,
            egui::DragValue::new(v).range(min_i..=max_i),
        );
        if dv.changed() {
            changed = true;
        }
    });

    changed
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo build 2>&1 | head -10`

- [ ] **Step 3: Run and verify visually**

Run: `cargo run --release`
Expected: Sliders in Color Adjustment node have thin 2px track with small solid circle handle. DragValue next to each slider for precise input.

- [ ] **Step 4: Commit**

```bash
git add src/node/widget/slider.rs
git commit -m "feat: custom StyledSlider with thin track and small handle"
```

---

## Chunk 6: Cleanup and Final Verification

### Task 14: Delete legacy nodes.rs and final cleanup

**Files:**
- Delete: `src/nodes.rs`

- [ ] **Step 1: Delete the untracked legacy file**

Run: `rm -f src/nodes.rs`

- [ ] **Step 2: Run full build + tests**

Run: `cargo build && cargo test 2>&1 | tail -20`
Expected: All clean.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy 2>&1 | tail -20`
Expected: No errors. Fix any warnings.

- [ ] **Step 4: Format code**

Run: `cargo fmt`

- [ ] **Step 5: Final visual verification**

Run: `cargo run --release`
Verify:
- App opens with light theme
- Cmd+T toggles to dark theme and back
- Nodes have aligned headers (no white gap)
- Output pin labels are right-aligned
- Load Image node is constrained width, path truncated
- Parameters have two-column layout
- Sliders have custom styling
- Preview panel works as before

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "chore: delete legacy nodes.rs, final cleanup"
```
