use std::collections::BTreeSet;

use crate::diagnostics::render_trace::{self, RenderTraceStage, TARGET_RENDER_TREE};
use crate::tree::{Tree, TreeDumpLevel, TreeSnapshot, TreeSnapshotMaxNodes, TreeSnapshotOptions};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TreeDumpPhase {
    Before,
    After,
    Manual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeDumpPoint {
    pub stage: RenderTraceStage,
    pub phase: TreeDumpPhase,
}

impl TreeDumpPoint {
    pub fn new(stage: RenderTraceStage, phase: TreeDumpPhase) -> Self {
        Self { stage, phase }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum TreeDumpTrigger {
    Off,
    Once,
    Invalid,
    Dirty,
    Frames,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeDumpConfig {
    trigger: TreeDumpTrigger,
    level: TreeDumpLevel,
    normal_max_nodes: usize,
    stages: BTreeSet<String>,
    frames: BTreeSet<u64>,
}

impl Default for TreeDumpConfig {
    fn default() -> Self {
        Self {
            trigger: TreeDumpTrigger::Off,
            level: TreeDumpLevel::Normal,
            normal_max_nodes: 500,
            stages: BTreeSet::new(),
            frames: BTreeSet::new(),
        }
    }
}

impl TreeDumpConfig {
    pub fn from_env() -> Self {
        let mut config = Self::default();
        if let Ok(value) = std::env::var("NODEIMG_TREE_DUMP") {
            config.trigger = parse_trigger(&value);
        }
        if let Ok(value) = std::env::var("NODEIMG_TREE_DUMP_LEVEL") {
            config.level = parse_level(&value);
        }
        if let Ok(value) = std::env::var("NODEIMG_TREE_DUMP_MAX_NODES") {
            if value.trim().eq_ignore_ascii_case("all") {
                config.normal_max_nodes = usize::MAX;
            } else if let Ok(limit) = value.trim().parse::<usize>() {
                config.normal_max_nodes = limit;
            }
        }
        if let Ok(value) = std::env::var("NODEIMG_TREE_DUMP_STAGES") {
            config.stages = parse_string_set(&value);
        }
        if let Ok(value) = std::env::var("NODEIMG_TREE_DUMP_FRAMES") {
            config.frames = parse_frame_set(&value);
        }
        config
    }

    fn options(&self) -> TreeSnapshotOptions {
        match self.level {
            TreeDumpLevel::Normal => {
                if self.normal_max_nodes == usize::MAX {
                    TreeSnapshotOptions {
                        level: TreeDumpLevel::Normal,
                        max_nodes: TreeSnapshotMaxNodes::All,
                    }
                } else {
                    TreeSnapshotOptions::normal(self.normal_max_nodes)
                }
            }
            TreeDumpLevel::Full => TreeSnapshotOptions::full(),
        }
    }

    fn stage_matches(&self, stage: RenderTraceStage) -> bool {
        self.stages.is_empty() || self.stages.contains(&format!("{stage:?}"))
    }
}

#[derive(Debug)]
pub struct TreeDumpController {
    config: TreeDumpConfig,
    dumped_once: bool,
}

impl TreeDumpController {
    pub fn from_env() -> Self {
        Self {
            config: TreeDumpConfig::from_env(),
            dumped_once: false,
        }
    }

    pub fn maybe_dump(&mut self, tree: &Tree, point: TreeDumpPoint, reason: &str) {
        if !render_trace::is_tree_debug_enabled() {
            return;
        }
        if !self.config.stage_matches(point.stage) {
            return;
        }

        let frame = render_trace::current_render_trace_frame();
        let needs_snapshot = match self.config.trigger {
            TreeDumpTrigger::Off => false,
            TreeDumpTrigger::Once => !self.dumped_once,
            TreeDumpTrigger::Frames => self.config.frames.contains(&frame.id),
            TreeDumpTrigger::Invalid | TreeDumpTrigger::Dirty => true,
        };
        if !needs_snapshot {
            return;
        }

        let snapshot = tree.debug_snapshot(self.config.options());
        let should_emit = match self.config.trigger {
            TreeDumpTrigger::Off => false,
            TreeDumpTrigger::Once | TreeDumpTrigger::Frames => true,
            TreeDumpTrigger::Invalid => snapshot.has_issues(),
            TreeDumpTrigger::Dirty => snapshot.has_dirty(),
        };
        if should_emit {
            emit_snapshot(&snapshot, point, reason);
            if self.config.trigger == TreeDumpTrigger::Once {
                self.dumped_once = true;
            }
        }
    }
}

pub fn emit_snapshot(snapshot: &TreeSnapshot, point: TreeDumpPoint, reason: &str) {
    let frame = render_trace::current_render_trace_frame();
    tracing::debug!(
        target: TARGET_RENDER_TREE,
        frame_id = frame.id,
        stage = ?point.stage,
        phase = ?point.phase,
        reason,
        summary = ?snapshot.summary,
        "tree dump summary"
    );
    for issue in &snapshot.issues {
        tracing::debug!(
            target: TARGET_RENDER_TREE,
            frame_id = frame.id,
            stage = ?point.stage,
            phase = ?point.phase,
            issue = %issue.message,
            "tree dump issue"
        );
    }
    for node in &snapshot.nodes {
        tracing::debug!(
            target: TARGET_RENDER_TREE,
            frame_id = frame.id,
            stage = ?point.stage,
            phase = ?point.phase,
            node = node.node,
            depth = node.depth,
            stable_id = %node.stable_id,
            line = %node.line,
            "tree dump node"
        );
    }
}

fn parse_trigger(value: &str) -> TreeDumpTrigger {
    match value.trim().to_ascii_lowercase().as_str() {
        "once" => TreeDumpTrigger::Once,
        "invalid" => TreeDumpTrigger::Invalid,
        "dirty" => TreeDumpTrigger::Dirty,
        "frames" => TreeDumpTrigger::Frames,
        _ => TreeDumpTrigger::Off,
    }
}

fn parse_level(value: &str) -> TreeDumpLevel {
    match value.trim().to_ascii_lowercase().as_str() {
        "full" => TreeDumpLevel::Full,
        _ => TreeDumpLevel::Normal,
    }
}

fn parse_string_set(value: &str) -> BTreeSet<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_frame_set(value: &str) -> BTreeSet<u64> {
    value
        .split(',')
        .filter_map(|part| part.trim().parse::<u64>().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_dump_config_parses_level_and_sets() {
        assert_eq!(parse_level("full"), TreeDumpLevel::Full);
        assert_eq!(parse_level("normal"), TreeDumpLevel::Normal);
        assert_eq!(
            parse_frame_set("1, 2, nope, 3")
                .into_iter()
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert!(parse_string_set("SceneSync, LayoutFlush").contains("SceneSync"));
    }
}
