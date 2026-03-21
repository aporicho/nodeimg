# UI Framework Design — Node Image Studio

## Overview

Refactor the UI layer to a component-based architecture with a dual-theme system (dark + light), fix visual defects in node rendering, and polish widget styles. The existing layout (left preview panel + central node editor) is preserved unchanged.

## 1. Component Architecture

### Current Problems

- `app.rs` `update()` contains all UI logic in one function
- `viewer.rs` (369 lines) handles node rendering, graph evaluation dispatch, and global state (preview texture) simultaneously
- No separation between UI components

### Target Structure

```
src/
├── app.rs                # App struct — assembles components, manages global state + theme toggle
├── ui/
│   ├── mod.rs            # Module exports
│   ├── preview_panel.rs  # PreviewPanel — left preview panel
│   └── node_canvas.rs    # NodeCanvas — wraps SnarlWidget + SnarlStyle
├── theme/
│   ├── mod.rs            # Theme trait definition + current theme management
│   ├── tokens.rs         # Design tokens (spacing, corner radius, font size) — shared by both themes
│   ├── dark.rs           # DarkTheme — Modern Neutral palette (full independent color set)
│   └── light.rs          # LightTheme — Refined Light palette (full independent color set)
├── node/                 # Existing node system
│   ├── viewer.rs         # SnarlViewer impl (node rendering only)
│   └── ...
```

### Theme Access in NodeViewer

`NodeViewer` implements the `SnarlViewer<NodeInstance>` trait (defined by egui-snarl), whose method signatures are fixed and cannot accept a `&dyn Theme` parameter. To give `NodeViewer` access to the current theme, it stores an `Arc<dyn Theme>` field:

```rust
pub struct NodeViewer {
    pub theme: Arc<dyn Theme>,
    // ... existing fields
}
```

When the user toggles themes, `App` updates `viewer.theme` with the new `Arc<dyn Theme>`. This is cheap (Arc clone) and avoids lifetime issues with the SnarlViewer trait methods. All theme-dependent calls inside `show_input`, `show_output`, `header_frame`, `node_frame`, and `show_body` use `self.theme.*()` instead of the old `crate::node::theme::*` constants.

### Pin Color Ownership

Currently pin colors are defined in two places: `DataTypeInfo.pin_color` in `DataTypeRegistry` (actually used) and `PIN_*` constants in `node/theme.rs` (vestigial, unused). After the refactor:

- **Pin colors move entirely to the Theme trait** via `fn pin_color(&self, data_type: &str) -> Color32`
- `DataTypeInfo.pin_color` field is removed from `DataTypeRegistry`
- `NodeViewer::pin_color()` method calls `self.theme.pin_color()` instead of looking up `DataTypeRegistry`
- This allows each theme to define its own pin color palette (brighter pins on dark backgrounds, etc.)

### Component Pattern

Each UI component follows the same pattern:

```rust
pub struct PreviewPanel {
    show: bool,
    // component-local state
}

impl PreviewPanel {
    pub fn new() -> Self { ... }

    pub fn show(&mut self, ctx: &egui::Context, theme: &dyn Theme, texture: Option<&TextureHandle>) {
        // All rendering logic for this panel
    }
}
```

`App::update()` becomes an assembler:

```rust
fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    self.theme.apply(ctx);
    self.preview_panel.show(ctx, &*self.theme, self.viewer.preview_texture.as_ref());
    // NodeCanvas includes the preview panel toggle button when panel is hidden
    self.node_canvas.show(ctx, &*self.theme, &mut self.snarl, &mut self.viewer,
                          &mut self.preview_panel);
}
```

### What Moves Where

| Current Location | Target | Content |
|---|---|---|
| `app.rs` lines 44-77 | `PreviewPanel::show()` | Preview panel rendering |
| `app.rs` lines 80-95 | `NodeCanvas::show()` | Node editor panel rendering |
| `app.rs` lines 19-33 | `App::new()` | Stays, but creates components |
| `node/theme.rs` (all) | `theme/` module | Replaced by Theme trait + implementations |

## 2. Theme System

### Theme Trait

```rust
pub trait Theme {
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

    // Apply full theme to egui context (visuals, style overrides)
    fn apply(&self, ctx: &egui::Context);

    // Node frames (used by SnarlViewer)
    fn node_header_frame(&self, cat_id: &str) -> Frame;
    fn node_body_frame(&self) -> Frame;
}
```

### Design Tokens (tokens.rs)

Layout constants shared by both themes:

```rust
// Spacing
pub const ITEM_SPACING: Vec2 = vec2(6.0, 4.0);
pub const BUTTON_PADDING: Vec2 = vec2(8.0, 4.0);
pub const NODE_HEADER_PADDING: Margin = Margin::symmetric(12, 6);
pub const NODE_BODY_PADDING: Margin = Margin::same(10);
pub const PARAM_LABEL_WIDTH: f32 = 80.0;

// Corner radius
pub const CORNER_RADIUS: f32 = 6.0;
pub const NODE_CORNER_RADIUS: f32 = 10.0;

// Font sizes
pub const FONT_SIZE_SMALL: f32 = 11.0;
pub const FONT_SIZE_NORMAL: f32 = 13.0;

// Node constraints
pub const NODE_MAX_WIDTH: f32 = 280.0;
pub const NODE_PREVIEW_MAX_SIZE: f32 = 200.0;
```

### DarkTheme (Modern Neutral)

- Canvas: `#2b2d30`
- Node body: `#393b40`, stroke `#4e5157`
- Panel: `#1e1f22`
- Text primary: `#b0b3b8`, secondary: `#6a6d73`
- Category colors: muted/desaturated variants suitable for dark backgrounds
- Pin colors: slightly brighter than light theme to maintain visibility

### LightTheme (Refined Light)

- Canvas: `#f0f1f5`
- Node body: `#ffffff`, stroke `#d8dae2`
- Panel: `#fafbfd`
- Text primary: `#1a1c24`, secondary: `#6b7080`
- Category colors: cleaner, more refined versions of current pastels
- Pin colors: similar to current but with better contrast on white

### Theme Switching

`App` holds `Arc<dyn Theme>` (shared with `NodeViewer`). Toggled by a keyboard shortcut (e.g., `Ctrl+T` or theme toggle button). The `apply()` method is called every frame unconditionally — setting egui visuals is cheap and avoids the complexity of dirty-tracking.

## 3. Node Visual Fixes

### 3.1 Header / Body Frame Alignment

**Problem:** The colored header bar renders inside the node body frame, causing a visible white gap between the header and the node's outer border. The header's corners don't align with the body's rounded corners.

**Fix:** Adjust the relationship between `node_frame` and `header_frame` in the SnarlViewer implementation:
- `node_frame` provides the outer shape (rounded corners, stroke, shadow) with zero inner padding at the top so the header can sit flush
- `header_frame` uses matching top corner radii and zero outer margin to fill the full width of the outer frame
- The key is ensuring both frames share the same width and the header's top corners match the outer frame's top corners exactly

Specific approach:
```rust
// node_frame: outer container
fn node_body_frame(&self) -> Frame {
    Frame {
        inner_margin: Margin { top: 0, bottom: 10, left: 10, right: 10 },
        corner_radius: CornerRadius::same(NODE_CORNER_RADIUS),
        fill: self.node_body_fill(),
        stroke: self.node_body_stroke(),
        shadow: self.node_shadow(),
        ..Default::default()
    }
}

// header_frame: fills top of node
fn node_header_frame(&self, cat_id: &str) -> Frame {
    Frame {
        inner_margin: Margin::symmetric(12, 6),
        corner_radius: CornerRadius { nw: NODE_CORNER_RADIUS, ne: NODE_CORNER_RADIUS, sw: 0, se: 0 },
        fill: self.category_color(cat_id),
        stroke: Stroke::NONE,
        ..Default::default()
    }
}
```

**Implementation note:** The exact margin/padding values depend on how egui-snarl composes `node_frame` and `header_frame` internally. The approach above (zero top margin on node_frame + matching top corner radii on header_frame) is the best-effort starting point. During implementation, if egui-snarl's rendering pipeline doesn't support flush header placement, the fallback is to remove the colored header frame entirely and instead paint the header background manually within `show_header` using `ui.painter().rect_filled()` — drawing a colored rect behind the title text that respects the outer frame's top corners.

### 3.2 Output Pin Text Alignment

**Problem:** Output pin labels use default left alignment, while the pin circle is at the right edge. This causes text and circle to visually collide.

**Fix:** Use right-to-left layout in `show_output`:
```rust
fn show_output(&mut self, pin: &OutPin, ui: &mut Ui, snarl: &mut Snarl<NodeInstance>) -> PinInfo {
    let instance = &snarl[pin.id.node];
    if let Some(def) = self.node_registry.get(&instance.type_id) {
        if let Some(pin_def) = def.outputs.get(pin.id.output) {
            let color = self.theme.pin_color(&pin_def.data_type);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.colored_label(self.theme.text_secondary(), &pin_def.name);
            });
            return PinInfo::circle().with_fill(color);
        }
    }
    PinInfo::circle().with_fill(Color32::GRAY)
}
```

### 3.3 Node Max Width

**Problem:** Load Image node stretches to full canvas width due to long file paths.

**Fix:**
- Set `NODE_MAX_WIDTH` in tokens (280px)
- File picker widget truncates path display: show only filename, full path in tooltip
- Apply max width constraint in `show_body` via `ui.set_max_width(tokens::NODE_MAX_WIDTH)`

### 3.4 Parameter Layout

**Problem:** Parameter name and value are concatenated without structure ("path Select file... Browse").

**Fix:** Two-column layout in parameter rendering:
```rust
ui.horizontal(|ui| {
    ui.allocate_ui_with_layout(
        vec2(tokens::PARAM_LABEL_WIDTH, ui.available_height()),
        Layout::left_to_right(Align::Center),
        |ui| { ui.label(&param_def.name); }
    );
    // Remaining space for widget
    self.widget_registry.render(...);
});
```

## 4. Widget Style Polish

Button and general widget styling is applied through `Theme::apply()` by modifying egui's `Visuals` and `Style`. The slider requires a custom widget because egui does not expose track height or handle size through its style API.

### Slider (custom widget)

egui's built-in `Slider` does not support customizing track height or handle size. A custom slider widget (`StyledSlider`) is needed in `src/node/widget/slider.rs`:

- Track height: 2px (drawn via `ui.painter().rect_filled()`)
- Track fill: theme accent color for the active portion, muted color for inactive
- Handle: small solid circle (6px diameter, `ui.painter().circle_filled()`)
- Value display: DragValue to the right, matches theme widget background
- Interaction: reuse egui's `Sense::click_and_drag()` for input handling

The custom slider replaces egui's default `Slider` only within node parameter rendering. Other sliders in the app (if any) continue using egui defaults.

### Button

- Unified corner radius from tokens
- Consistent padding from tokens
- Hover state: subtle background color shift using `widget_hover_bg()`
- Active state: accent color fill

### File Picker

- Text field: truncated path display, full path on tooltip hover
- Browse button: same style as other buttons
- Proper spacing between text field and button

### Global Style Overrides in apply()

```rust
fn apply(&self, ctx: &egui::Context) {
    let base = if self.is_dark() { Visuals::dark() } else { Visuals::light() };
    ctx.set_visuals(base);

    ctx.style_mut(|style| {
        // Spacing
        style.spacing.item_spacing = tokens::ITEM_SPACING;
        style.spacing.button_padding = tokens::BUTTON_PADDING;
        style.spacing.slider_width = 120.0;

        // Corner radius
        let cr = CornerRadius::same(tokens::CORNER_RADIUS);
        style.visuals.widgets.noninteractive.corner_radius = cr;
        style.visuals.widgets.inactive.corner_radius = cr;
        style.visuals.widgets.hovered.corner_radius = cr;
        style.visuals.widgets.active.corner_radius = cr;

        // Widget fills, strokes, text colors — all from theme methods
        // ...
    });
}
```

## 5. Files Changed

| File | Action | Description |
|---|---|---|
| `src/ui/mod.rs` | New | Module exports |
| `src/ui/preview_panel.rs` | New | PreviewPanel component |
| `src/ui/node_canvas.rs` | New | NodeCanvas component |
| `src/theme/mod.rs` | New | Theme trait + manager |
| `src/theme/tokens.rs` | New | Shared design tokens |
| `src/theme/dark.rs` | New | DarkTheme implementation |
| `src/theme/light.rs` | New | LightTheme implementation |
| `src/app.rs` | Modified | Refactored to use components + theme |
| `src/node/theme.rs` | Deleted | Replaced by theme/ module |
| `src/node/viewer.rs` | Modified | Pin alignment fix, theme integration, max width |
| `src/node/widget/mod.rs` | Modified | Parameter two-column layout |
| `src/node/widget/slider.rs` | Modified | Custom StyledSlider widget with themed track and handle |
| `src/node/types.rs` | Modified | Remove `pin_color` field from `DataTypeInfo` |
| `src/node/widget/file_picker.rs` | Modified | Path truncation + tooltip |
| `src/lib.rs` | Modified | Add ui and theme module declarations |
| `src/nodes.rs` | Deleted | Legacy untracked file, no longer used |

## 6. Out of Scope

- Preview panel enhancements (zoom, info overlay)
- Status bar
- Undo/redo
- Project save/load UI
- New node types
- Canvas grid/guidelines
- Keyboard shortcuts (beyond theme toggle)
