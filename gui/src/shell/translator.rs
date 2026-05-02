use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key as WinitKey, KeyCode, NamedKey, PhysicalKey};

use super::event::{AppEvent, Key, Modifiers, MouseButton};

/// 有状态的事件翻译器。追踪光标位置和修饰键，将 winit 事件翻译为 AppEvent。
pub struct EventTranslator {
    cursor_x: f64,
    cursor_y: f64,
    scale_factor: f64,
    modifiers: Modifiers,
    ime_preedit_active: bool,
}

impl EventTranslator {
    pub fn new(scale_factor: f64) -> Self {
        Self {
            cursor_x: 0.0,
            cursor_y: 0.0,
            scale_factor,
            modifiers: Modifiers::default(),
            ime_preedit_active: false,
        }
    }

    /// 翻译 winit 事件。一个底层事件可能产出多个应用层事件。
    pub fn translate(&mut self, event: &WindowEvent) -> Vec<AppEvent> {
        match event {
            // ── 鼠标 ──
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_x = position.x;
                self.cursor_y = position.y;
                let (x, y) = self.logical_cursor();
                vec![AppEvent::MouseMove { x, y }]
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let Some(btn) = translate_button(button) else {
                    return Vec::new();
                };
                let (x, y) = self.logical_cursor();
                match state {
                    ElementState::Pressed => vec![AppEvent::MousePress { x, y, button: btn }],
                    ElementState::Released => vec![AppEvent::MouseRelease { x, y, button: btn }],
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (x, y) = self.logical_cursor();
                match delta {
                    MouseScrollDelta::LineDelta(dx, dy) => vec![AppEvent::ScrollLine {
                        x,
                        y,
                        delta_x: *dx,
                        delta_y: *dy,
                    }],
                    MouseScrollDelta::PixelDelta(pos) => vec![AppEvent::ScrollPixel {
                        x,
                        y,
                        delta_x: pos.x as f32,
                        delta_y: pos.y as f32,
                    }],
                }
            }

            // ── 键盘 ──
            WindowEvent::ModifiersChanged(mods) => {
                let state = mods.state();
                self.modifiers = Modifiers {
                    shift: state.shift_key(),
                    ctrl: state.control_key(),
                    alt: state.alt_key(),
                    meta: state.super_key(),
                };
                Vec::new()
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let key = translate_key(event);
                match event.state {
                    ElementState::Pressed => {
                        let mut events = vec![AppEvent::KeyPress {
                            key,
                            modifiers: self.modifiers,
                        }];
                        let shortcut_modifier = self.modifiers.ctrl || self.modifiers.meta;
                        if !self.ime_preedit_active && !shortcut_modifier {
                            if let Some(text) = translate_text(event) {
                                events.push(AppEvent::TextInput { text });
                            }
                        }
                        events
                    }
                    ElementState::Released => vec![AppEvent::KeyRelease {
                        key,
                        modifiers: self.modifiers,
                    }],
                }
            }
            WindowEvent::Ime(winit::event::Ime::Enabled) => Vec::new(),
            WindowEvent::Ime(winit::event::Ime::Preedit(text, caret)) => {
                self.ime_preedit_active = !text.is_empty();
                vec![AppEvent::ImePreedit {
                    text: text.clone(),
                    caret: *caret,
                }]
            }
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                self.ime_preedit_active = false;
                vec![AppEvent::TextInput { text: text.clone() }]
            }
            WindowEvent::Ime(winit::event::Ime::Disabled) => {
                self.ime_preedit_active = false;
                vec![AppEvent::ImePreedit {
                    text: String::new(),
                    caret: None,
                }]
            }

            // ── 窗口 ──
            WindowEvent::Resized(size) => vec![AppEvent::Resized {
                width: size.width,
                height: size.height,
            }],
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = *scale_factor;
                vec![AppEvent::ScaleFactorChanged {
                    scale_factor: *scale_factor,
                }]
            }
            WindowEvent::CloseRequested => vec![AppEvent::CloseRequested],
            WindowEvent::Focused(focused) => {
                if *focused {
                    vec![AppEvent::Focused]
                } else {
                    vec![AppEvent::Unfocused]
                }
            }
            WindowEvent::CursorEntered { .. } => {
                tracing::trace!(target: "gui::cursor", "winit cursor entered window");
                Vec::new()
            }
            WindowEvent::CursorLeft { .. } => {
                tracing::trace!(target: "gui::cursor", "winit cursor left window");
                Vec::new()
            }

            // ── 触控板手势 ──
            WindowEvent::PinchGesture { delta, .. } => {
                let (x, y) = self.logical_cursor();
                vec![AppEvent::PinchZoom {
                    x,
                    y,
                    delta: *delta as f32,
                }]
            }

            _ => Vec::new(),
        }
    }

    fn logical_cursor(&self) -> (f32, f32) {
        (
            (self.cursor_x / self.scale_factor) as f32,
            (self.cursor_y / self.scale_factor) as f32,
        )
    }
}

fn translate_button(button: &winit::event::MouseButton) -> Option<MouseButton> {
    match button {
        winit::event::MouseButton::Left => Some(MouseButton::Left),
        winit::event::MouseButton::Right => Some(MouseButton::Right),
        winit::event::MouseButton::Middle => Some(MouseButton::Middle),
        _ => None,
    }
}

fn translate_text(event: &winit::event::KeyEvent) -> Option<String> {
    let text = event.text.as_ref()?;
    if text.is_empty() || text.chars().all(char::is_control) {
        return None;
    }
    Some(text.to_string())
}

fn translate_key(event: &winit::event::KeyEvent) -> Key {
    let physical = match event.physical_key {
        PhysicalKey::Code(code) => translate_key_code(code),
        _ => Key::Other,
    };
    if physical != Key::Other {
        return physical;
    }

    translate_logical_key(&event.logical_key)
}

fn translate_key_code(code: KeyCode) -> Key {
    match code {
        KeyCode::Escape => Key::Escape,
        KeyCode::Tab => Key::Tab,
        KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::ArrowLeft => Key::Left,
        KeyCode::ArrowRight => Key::Right,
        KeyCode::ArrowUp => Key::Up,
        KeyCode::ArrowDown => Key::Down,
        KeyCode::Space => Key::Space,
        KeyCode::F1 => Key::Function(1),
        KeyCode::F2 => Key::Function(2),
        KeyCode::F3 => Key::Function(3),
        KeyCode::F4 => Key::Function(4),
        KeyCode::F5 => Key::Function(5),
        KeyCode::F6 => Key::Function(6),
        KeyCode::F7 => Key::Function(7),
        KeyCode::F8 => Key::Function(8),
        KeyCode::F9 => Key::Function(9),
        KeyCode::F10 => Key::Function(10),
        KeyCode::F11 => Key::Function(11),
        KeyCode::F12 => Key::Function(12),
        KeyCode::KeyA => Key::Char('A'),
        KeyCode::KeyB => Key::Char('B'),
        KeyCode::KeyC => Key::Char('C'),
        KeyCode::KeyD => Key::Char('D'),
        KeyCode::KeyE => Key::Char('E'),
        KeyCode::KeyF => Key::Char('F'),
        KeyCode::KeyG => Key::Char('G'),
        KeyCode::KeyH => Key::Char('H'),
        KeyCode::KeyI => Key::Char('I'),
        KeyCode::KeyJ => Key::Char('J'),
        KeyCode::KeyK => Key::Char('K'),
        KeyCode::KeyL => Key::Char('L'),
        KeyCode::KeyM => Key::Char('M'),
        KeyCode::KeyN => Key::Char('N'),
        KeyCode::KeyO => Key::Char('O'),
        KeyCode::KeyP => Key::Char('P'),
        KeyCode::KeyQ => Key::Char('Q'),
        KeyCode::KeyR => Key::Char('R'),
        KeyCode::KeyS => Key::Char('S'),
        KeyCode::KeyT => Key::Char('T'),
        KeyCode::KeyU => Key::Char('U'),
        KeyCode::KeyV => Key::Char('V'),
        KeyCode::KeyW => Key::Char('W'),
        KeyCode::KeyX => Key::Char('X'),
        KeyCode::KeyY => Key::Char('Y'),
        KeyCode::KeyZ => Key::Char('Z'),
        _ => Key::Other,
    }
}

fn translate_logical_key(logical_key: &WinitKey) -> Key {
    match logical_key {
        WinitKey::Named(named) => translate_named_key(*named),
        WinitKey::Character(text) => text
            .chars()
            .next()
            .filter(|ch| ch.is_ascii_alphabetic())
            .map(|ch| Key::Char(ch.to_ascii_uppercase()))
            .unwrap_or(Key::Other),
        _ => Key::Other,
    }
}

fn translate_named_key(named: NamedKey) -> Key {
    match named {
        NamedKey::Escape => Key::Escape,
        NamedKey::Tab => Key::Tab,
        NamedKey::Enter => Key::Enter,
        NamedKey::Backspace => Key::Backspace,
        NamedKey::Delete => Key::Delete,
        NamedKey::Home => Key::Home,
        NamedKey::End => Key::End,
        NamedKey::ArrowLeft => Key::Left,
        NamedKey::ArrowRight => Key::Right,
        NamedKey::ArrowUp => Key::Up,
        NamedKey::ArrowDown => Key::Down,
        NamedKey::Space => Key::Space,
        NamedKey::F1 => Key::Function(1),
        NamedKey::F2 => Key::Function(2),
        NamedKey::F3 => Key::Function(3),
        NamedKey::F4 => Key::Function(4),
        NamedKey::F5 => Key::Function(5),
        NamedKey::F6 => Key::Function(6),
        NamedKey::F7 => Key::Function(7),
        NamedKey::F8 => Key::Function(8),
        NamedKey::F9 => Key::Function(9),
        NamedKey::F10 => Key::Function(10),
        NamedKey::F11 => Key::Function(11),
        NamedKey::F12 => Key::Function(12),
        _ => Key::Other,
    }
}
