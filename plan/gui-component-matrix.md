# GUI Component Matrix

**目的：** 记录当前基础控件库的稳定面、展示覆盖和已知缺口，并与 `app/src/demo_gallery.rs` 的 gallery 分区保持一致。

## 状态定义

- `Stable`：接口和行为已进入收敛阶段，后续以修复、统一和测试补齐为主
- `Experimental`：可用，但仍可能在交互、视觉或框架边界上继续调整

## Typography

### Label
- Status: Stable
- Gallery Coverage: `Typography`
- Supported:
  - `Body / Caption / Title`
  - `family / line_height / weight / italic` 通过 `TextStyle` 生效
  - muted/disabled 颜色
- Missing:
  - rich text
  - wrapping / ellipsis / max_lines

### TextStyle Showcase
- Status: Stable
- Gallery Coverage: `Typography`
- Supported:
  - sans / monospace
  - regular / semibold / bold override
  - italic
- Missing:
  - custom font family registration
  - runtime theme typography switching

## Buttons

### Button
- Status: Stable
- Gallery Coverage: `Buttons`
- Supported:
  - click
  - disabled
  - hover / pressed / focused visuals
  - long label display
- Missing:
  - icon support still未落地

## Inputs

### TextInput
- Status: Stable
- Gallery Coverage: `Inputs`
- Supported:
  - single-line editing
  - caret / selection / drag selection
  - clipboard
  - IME preedit / commit
  - long text horizontal scroll and clip
  - `Home / End`
- Missing:
  - rich formatting
  - multi-line
  - wrap / ellipsis / max_lines

### NumberInput
- Status: Stable
- Gallery Coverage: `Inputs`
- Supported:
  - parse / revert / invalid editing state
  - min / max / step / precision
  - `Up / Down` stepping
  - monospace + medium visual
- Missing:
  - spinner buttons
  - drag-to-adjust

### Slider
- Status: Stable
- Gallery Coverage: `Inputs`
- Supported:
  - click / drag
  - step snapping
  - value display
- Missing:
  - keyboard adjustment

## Selection

### Toggle
- Status: Stable
- Gallery Coverage: `Selection`
- Supported:
  - click
  - state visuals
- Missing:
  - keyboard toggle path

### Checkbox
- Status: Stable
- Gallery Coverage: `Selection`
- Supported:
  - click
  - checked / disabled visuals
- Missing:
  - keyboard toggle path

### Radio
- Status: Stable
- Gallery Coverage: `Selection`
- Supported:
  - click selection
  - selected / disabled visuals
- Missing:
  - framework-owned radio group semantics

### Dropdown / Select
- Status: Experimental
- Gallery Coverage: `Selection`
- Supported:
  - click open / close
  - option click select
  - keyboard open / move / confirm
  - outside click dismiss
  - Escape dismiss
  - focus restore
- Missing:
  - searchable mode
  - multi-select
  - richer popup styling

## Containers

### Group / Section
- Status: Stable
- Gallery Coverage: `Containers`, plus section wrappers across gallery
- Supported:
  - titled content grouping
  - themed container chrome
- Missing:
  - compact / borderless variants

### Collapsible
- Status: Stable
- Gallery Coverage: `Containers`
- Supported:
  - controlled expanded state
  - header click
  - collapsed content removal from tree
- Missing:
  - uncontrolled mode
  - animation

### Panel
- Status: Stable
- Gallery Coverage: root demo container
- Supported:
  - drag
  - resize
  - auto height when `h <= 0`
- Missing:
  - scrollable title/content split

## Scrolling & Media

### ScrollArea
- Status: Stable
- Gallery Coverage: `Scrolling & Media`
- Supported:
  - wheel routing
  - `overflow: Scroll`
  - clip
- Missing:
  - visible scrollbar
  - nested scroll policy tuning

### ListView
- Status: Stable
- Gallery Coverage: `Scrolling & Media`
- Supported:
  - long vertical list built on `ScrollArea`
- Missing:
  - selection model
  - virtualization

### ImageViewer
- Status: Experimental
- Gallery Coverage: `Scrolling & Media`
- Supported:
  - texture display
  - theme-backed frame
- Missing:
  - fit modes
  - image interaction tools

## Overlay

### Popup Host
- Status: Experimental
- Gallery Coverage: `Overlay`
- Supported:
  - single active overlay
  - anchored placement
  - outside click dismiss
  - Escape dismiss
  - focus restore
- Missing:
  - stacked overlays
  - popup-specific theme tokens
  - initial focus handoff strategy

## 收敛准则

一个控件要从 `Experimental` 进入 `Stable`，至少需要满足：

1. gallery 中有正常态、禁用态或边界态展示
2. 有明确语义 action，而不只依赖底层 click id
3. 有 framework 级回归测试
4. 不需要再修改 `tree` core 才能继续修它
