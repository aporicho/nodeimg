use super::recognizer::{GestureDisposition, GestureRecognizer};
use super::signal::GestureSignal;

pub struct GestureArena {
    members: Vec<Box<dyn GestureRecognizer>>,
    resolved: bool,
    winner_signal: Option<GestureSignal>,
}

impl GestureArena {
    pub fn new() -> Self {
        Self {
            members: Vec::new(),
            resolved: false,
            winner_signal: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    pub(crate) fn add(&mut self, recognizer: Box<dyn GestureRecognizer>) {
        self.members.push(recognizer);
    }

    pub(crate) fn pointer_move(&mut self, x: f32, y: f32) -> Option<GestureSignal> {
        if self.resolved {
            return None;
        }

        // 已有赢家持续产出 GestureSignal
        if self.winner_signal.is_some() {
            if let Some(member) = self.members.first_mut() {
                let disp = member.on_pointer_move(x, y);
                if disp == GestureDisposition::Accepted {
                    return Some(member.accept());
                }
            }
            return None;
        }

        let mut accepted_idx = None;
        let mut i = 0;
        while i < self.members.len() {
            let disp = self.members[i].on_pointer_move(x, y);
            match disp {
                GestureDisposition::Rejected => {
                    self.members[i].reject();
                    self.members.remove(i);
                }
                GestureDisposition::Accepted => {
                    accepted_idx = Some(i);
                    break;
                }
                GestureDisposition::Pending => {
                    i += 1;
                }
            }
        }

        if let Some(idx) = accepted_idx {
            return Some(self.resolve_winner(idx));
        }
        None
    }

    pub(crate) fn pointer_up(&mut self, x: f32, y: f32) -> Option<GestureSignal> {
        if self.resolved {
            return None;
        }

        if self.winner_signal.is_some() {
            if let Some(member) = self.members.first_mut() {
                member.on_pointer_up(x, y);
                let signal = member.accept();
                self.resolved = true;
                return Some(signal);
            }
            return None;
        }

        let mut accepted_idx = None;
        let mut i = 0;
        while i < self.members.len() {
            let disp = self.members[i].on_pointer_up(x, y);
            match disp {
                GestureDisposition::Rejected => {
                    self.members[i].reject();
                    self.members.remove(i);
                }
                GestureDisposition::Accepted => {
                    accepted_idx = Some(i);
                    break;
                }
                GestureDisposition::Pending => {
                    i += 1;
                }
            }
        }

        if let Some(idx) = accepted_idx {
            let signal = self.resolve_winner(idx);
            self.resolved = true;
            return Some(signal);
        }

        if let Some(signal) = self.try_auto_resolve() {
            self.resolved = true;
            return Some(signal);
        }

        if self.members.is_empty() {
            self.resolved = true;
        }
        None
    }

    fn resolve_winner(&mut self, winner_idx: usize) -> GestureSignal {
        for (i, member) in self.members.iter_mut().enumerate() {
            if i != winner_idx {
                member.reject();
            }
        }
        let winner = self.members.swap_remove(winner_idx);
        self.members.clear();
        self.members.push(winner);
        let signal = self.members[0].accept();
        self.winner_signal = Some(signal.clone());
        signal
    }

    fn try_auto_resolve(&mut self) -> Option<GestureSignal> {
        if self.members.len() == 1 {
            let signal = self.members[0].accept();
            self.winner_signal = Some(signal.clone());
            Some(signal)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gesture::{DragRecognizer, GestureRecognizer, TapRecognizer};
    use std::time::Instant;

    #[test]
    fn tap_is_emitted_only_once_on_pointer_up() {
        let mut arena = GestureArena::new();
        let mut tap = TapRecognizer::new(
            "toggle_grid::track".to_string(),
            Some(Instant::now() - std::time::Duration::from_secs(1)),
        );
        assert!(tap.on_pointer_down(10.0, 10.0));
        arena.add(Box::new(tap));

        assert!(arena.pointer_move(10.5, 10.5).is_none());
        let signal = arena.pointer_up(10.5, 10.5);
        assert!(matches!(signal, Some(GestureSignal::Click(_))));
        assert!(arena.pointer_up(10.5, 10.5).is_none());
    }

    #[test]
    fn deepest_drag_member_wins_over_parent_drag() {
        let mut arena = GestureArena::new();
        let mut child = DragRecognizer::new("child".to_string());
        let mut parent = DragRecognizer::new("parent".to_string());
        assert!(child.on_pointer_down(0.0, 0.0));
        assert!(parent.on_pointer_down(0.0, 0.0));
        arena.add(Box::new(child));
        arena.add(Box::new(parent));

        let signal = arena.pointer_move(8.0, 0.0).expect("drag start");

        assert!(matches!(
            signal,
            GestureSignal::DragStart { id, .. } if id == "child"
        ));
    }
}
