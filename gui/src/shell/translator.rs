use winit::event::{ElementState, MouseScrollDelta, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};

use super::event::{AppEvent, Key, Modifiers, MouseButton};

/// 有状态的事件翻译器。追踪光标位置和修饰键，将 winit 事件翻译为 AppEvent。
pub struct EventTranslator {
    cursor_x: f64,
    cursor_y: f64,
    scale_factor: f64,
    modifiers: Modifiers,
}

impl EventTranslator {
    pub fn new(scale_factor: f64) -> Self {
        Self {
            cursor_x: 0.0,
            cursor_y: 0.0,
            scale_factor,
            modifiers: Modifiers::default(),
        }
    }

    /// 翻译 winit 事件。返回 None 表示该事件不需要传递给应用层。
    pub fn translate(&mut self, event: &WindowEvent) -> Option<AppEvent> {
        match event {
            // ── 鼠标 ──
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_x = position.x;
                self.cursor_y = position.y;
                let (x, y) = self.logical_cursor();
                Some(AppEvent::MouseMove { x, y })
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let btn = translate_button(button)?;
                let (x, y) = self.logical_cursor();
                match state {
                    ElementState::Pressed => Some(AppEvent::MousePress { x, y, button: btn }),
                    ElementState::Released => Some(AppEvent::MouseRelease { x, y, button: btn }),
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (*x * 40.0, *y * 40.0),
                    MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                };
                let (x, y) = self.logical_cursor();
                Some(AppEvent::Scroll { x, y, delta_x: dx, delta_y: dy })
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
                None
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let key = translate_key(event);
                match event.state {
                    ElementState::Pressed => Some(AppEvent::KeyPress { key, modifiers: self.modifiers }),
                    ElementState::Released => Some(AppEvent::KeyRelease { key, modifiers: self.modifiers }),
                }
            }
            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                Some(AppEvent::TextInput { text: text.clone() })
            }

            // ── 窗口 ──
            WindowEvent::Resized(size) => {
                Some(AppEvent::Resized { width: size.width, height: size.height })
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale_factor = *scale_factor;
                Some(AppEvent::ScaleFactorChanged { scale_factor: *scale_factor })
            }
            WindowEvent::CloseRequested => Some(AppEvent::CloseRequested),
            WindowEvent::Focused(focused) => {
                if *focused { Some(AppEvent::Focused) } else { Some(AppEvent::Unfocused) }
            }

            // ── 触控板手势 ──
            WindowEvent::PinchGesture { delta, .. } => {
                Some(AppEvent::PinchZoom { delta: *delta as f32 })
            }

            _ => None,
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

fn translate_key(event: &winit::event::KeyEvent) -> Key {
    match event.physical_key {
        PhysicalKey::Code(code) => match code {
            KeyCode::Escape => Key::Escape,
            KeyCode::Tab => Key::Tab,
            KeyCode::Enter | KeyCode::NumpadEnter => Key::Enter,
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Delete => Key::Delete,
            KeyCode::ArrowLeft => Key::Left,
            KeyCode::ArrowRight => Key::Right,
            KeyCode::ArrowUp => Key::Up,
            KeyCode::ArrowDown => Key::Down,
            KeyCode::Space => Key::Space,
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
        },
        _ => Key::Other,
    }
}
