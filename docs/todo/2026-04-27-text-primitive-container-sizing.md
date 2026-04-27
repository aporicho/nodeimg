# Text Primitive and Container Sizing

## Goal

Unify editable text around a single text primitive inspired by Figma's text box model:

- Text owns content, layout, wrapping, selection, caret, IME, and intrinsic size.
- Field widgets own background, border, padding, focus visuals, and disabled state.
- Containers consume absolute intrinsic sizes instead of growth deltas.

## Figma Mapping

- Text auto width: content controls width and height, no soft wrapping.
- Text auto height: parent controls width, text wraps, height follows line count.
- Text fixed size: parent controls width and height, overflow policy handles excess content.
- Container hug: parent size follows children.
- Container fill: child takes assigned space and can shrink.
- Container fixed: size is independent from child content.

## Checklist

- [x] Move shared text editing state into `gui/src/text/edit.rs`.
- [x] Move shared wrapped text layout into `gui/src/text/layout.rs`.
- [x] Add `TextIntrinsic` with absolute current/min/desired size fields.
- [x] Make TextArea value leaf an anchor instead of using real content to drive layout.
- [x] Change canvas node sizing from growth delta to absolute target size.
- [x] Persist canvas node `user_min_height` so manual vertical resize has explicit semantics.
- [x] Make macOS trackpad pixel scrolling move canvas content with the finger delta.
- [x] Introduce `TextBoxProps` as the public editable text primitive.
- [x] Rebuild TextInput, TextArea, and NumberInput atom shells through `TextBoxProps`.
- [x] Let runtime, systems, and painters accept direct single-line and multiline `TextBoxProps`.
- [x] Migrate NumberInput to reuse single-line TextBox while keeping numeric behavior outside text runtime.
- [ ] Standardize public container axis sizing names around Hug / Fill / Fixed.
- [x] Collapse TextInputRuntime and TextAreaRuntime into one TextBoxRuntime once single-line scroll and multiline wrap policies share one interface.
- [x] Remove old TextInput/TextArea duplicate runtime and painter wrapper implementations after TextBoxRuntime lands.

## Acceptance Tests

- [x] `cargo test -p gui text_area`
- [x] `cargo test -p gui text_box`
- [x] `cargo test -p gui text_input`
- [x] `cargo test -p gui number_input`
- [x] `cargo test -p gui node_sizing`
- [x] `cargo test -p gui pixel_scroll`
- [x] `cargo test -p app project_layout`
- [x] `cargo test -p gui`
- [x] `cargo test -p app`
