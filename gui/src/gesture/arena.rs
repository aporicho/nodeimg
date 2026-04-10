use super::recognizer::{GestureDisposition, GestureRecognizer};
use crate::widget::action::Action;

pub struct GestureArena {
    members: Vec<Box<dyn GestureRecognizer>>,
    target_id: String,
    resolved: bool,
    winner_action: Option<Action>,
}

impl GestureArena {
    pub fn new(target_id: String) -> Self {
        Self { members: Vec::new(), target_id, resolved: false, winner_action: None }
    }

    pub fn target_id(&self) -> &str { &self.target_id }
    pub fn is_empty(&self) -> bool { self.members.is_empty() }
    pub fn is_resolved(&self) -> bool { self.resolved }

    pub fn add(&mut self, recognizer: Box<dyn GestureRecognizer>) {
        self.members.push(recognizer);
    }

    pub fn pointer_move(&mut self, x: f32, y: f32) -> Option<Action> {
        if self.resolved { return None; }

        // 已有赢家持续产出 Action
        if self.winner_action.is_some() {
            if let Some(member) = self.members.first_mut() {
                let disp = member.on_pointer_move(x, y);
                if disp == GestureDisposition::Accepted {
                    return Some(member.accept());
                }
            }
            return None;
        }

        let mut accepted_idx = None;
        let mut i = self.members.len();
        while i > 0 {
            i -= 1;
            let disp = self.members[i].on_pointer_move(x, y);
            match disp {
                GestureDisposition::Rejected => {
                    self.members[i].reject();
                    self.members.remove(i);
                }
                GestureDisposition::Accepted => { accepted_idx = Some(i); break; }
                GestureDisposition::Pending => {}
            }
        }

        if let Some(idx) = accepted_idx {
            return Some(self.resolve_winner(idx));
        }
        self.try_auto_resolve()
    }

    pub fn pointer_up(&mut self, x: f32, y: f32) -> Option<Action> {
        if self.resolved { return None; }

        if self.winner_action.is_some() {
            if let Some(member) = self.members.first_mut() {
                member.on_pointer_up(x, y);
                let action = member.accept();
                self.resolved = true;
                return Some(action);
            }
            return None;
        }

        let mut accepted_idx = None;
        let mut i = self.members.len();
        while i > 0 {
            i -= 1;
            let disp = self.members[i].on_pointer_up(x, y);
            match disp {
                GestureDisposition::Rejected => {
                    self.members[i].reject();
                    self.members.remove(i);
                }
                GestureDisposition::Accepted => { accepted_idx = Some(i); break; }
                GestureDisposition::Pending => {}
            }
        }

        if let Some(idx) = accepted_idx {
            let action = self.resolve_winner(idx);
            self.resolved = true;
            return Some(action);
        }

        if let Some(action) = self.try_auto_resolve() {
            self.resolved = true;
            return Some(action);
        }

        if self.members.is_empty() { self.resolved = true; }
        None
    }

    fn resolve_winner(&mut self, winner_idx: usize) -> Action {
        for (i, member) in self.members.iter_mut().enumerate() {
            if i != winner_idx { member.reject(); }
        }
        let winner = self.members.swap_remove(winner_idx);
        self.members.clear();
        self.members.push(winner);
        let action = self.members[0].accept();
        self.winner_action = Some(action.clone());
        action
    }

    fn try_auto_resolve(&mut self) -> Option<Action> {
        if self.members.len() == 1 {
            let action = self.members[0].accept();
            self.winner_action = Some(action.clone());
            Some(action)
        } else {
            None
        }
    }
}
