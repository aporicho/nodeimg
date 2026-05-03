use crate::panels::EnginePanelState;
use crate::workspace::node_palette::{NodePaletteItem, NodePaletteState};
use engine::events::ExecutionStatus;
use engine::facade::EngineFacade;
use engine::Engine;
use gui::canvas::node_template::{
    CanvasNodeInstanceState, CanvasNodeParamTemplate, CanvasNodePortState, CanvasNodePortTemplate,
    CanvasNodeRenderView, CanvasNodeTemplate,
};
use gui::canvas::{
    canvas_port_stable_id, CanvasConnectionView, CanvasNodeIdentity, CanvasNodeLayout,
    CanvasPortConnectionState, CanvasPortSide,
};
use gui::control::{ControlSpec, ControlSpecMap};
use gui::renderer::Rect;
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct CanvasNodeTemplateCache {
    templates: HashMap<CanvasNodeTemplateKey, CanvasNodeTemplate>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct CanvasNodeTemplateKey {
    type_id: String,
    version: u32,
}

const CANVAS_NODE_WIDTH: f32 = 304.0;
const CANVAS_NODE_MIN_HEIGHT: f32 = 96.0;
const CANVAS_NODE_PADDING_Y: f32 = 48.0;
const CANVAS_NODE_PARAM_ROW_HEIGHT: f32 = 36.0;
const CANVAS_NODE_ROW_GAP_Y: f32 = 12.0;
const CANVAS_NODE_PIN_ROW_HEIGHT: f32 = 32.0;
const CANVAS_NODE_PIN_ROW_GAP: f32 = 12.0;
const CANVAS_NODE_COLUMN_GAP: f32 = 680.0;
const CANVAS_NODE_ROW_GAP: f32 = 180.0;
const CANVAS_NODE_COLUMNS: usize = 3;

impl CanvasNodeTemplateCache {
    fn template_for_def(&mut self, def: &engine::node_manager::NodeDef) -> CanvasNodeTemplate {
        let key = CanvasNodeTemplateKey {
            type_id: def.type_id.clone(),
            version: def.version,
        };
        self.templates
            .entry(key)
            .or_insert_with(|| canvas_node_template(def))
            .clone()
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.templates.len()
    }
}

pub(crate) fn engine_panel_state(engine: &Engine, last_action: &str) -> EnginePanelState {
    let graph = engine.query_graph_snapshot();
    let summary = engine.graph_state_summary();
    let execution_status = match engine.execution_state().status {
        ExecutionStatus::Idle => "Idle",
        ExecutionStatus::Running => "Running",
        ExecutionStatus::Cancelling => "Cancelling",
    };

    EnginePanelState {
        node_count: graph.nodes.len(),
        connection_count: graph.connections.len(),
        node_def_count: engine.list_node_defs().len(),
        graph_version: summary.graph_version,
        dirty: summary.dirty,
        execution_status: execution_status.to_string(),
        last_action: last_action.to_string(),
    }
}

pub(crate) fn node_palette_state(engine: &Engine) -> NodePaletteState {
    let mut items = engine
        .list_node_defs()
        .into_iter()
        .map(|node| NodePaletteItem {
            type_id: node.type_id.clone(),
            name: node.name.clone(),
            category: node.category.clone(),
            source: format!("{:?}", node.source),
        })
        .collect::<Vec<_>>();
    items.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.type_id.cmp(&b.type_id))
    });

    NodePaletteState { items }
}

pub(crate) fn canvas_node_identities(engine: &Engine) -> Vec<CanvasNodeIdentity> {
    let graph = engine.query_graph_snapshot();
    let mut node_ids = graph.nodes.keys().copied().collect::<Vec<_>>();
    node_ids.sort_by_key(|id| id.0);

    node_ids
        .into_iter()
        .enumerate()
        .map(|(index, node_id)| CanvasNodeIdentity {
            owner_id: engine_node_owner_id(node_id),
            default_rect: default_canvas_node_rect(index),
        })
        .collect()
}

#[cfg(test)]
pub(crate) fn canvas_node_templates(engine: &Engine) -> HashMap<String, CanvasNodeTemplate> {
    engine
        .list_node_defs()
        .into_iter()
        .map(|def| (def.type_id.clone(), canvas_node_template(def)))
        .collect()
}

fn canvas_node_template(def: &engine::node_manager::NodeDef) -> CanvasNodeTemplate {
    CanvasNodeTemplate {
        type_id: def.type_id.clone(),
        title: def.name.clone(),
        subtitle: def.type_id.clone(),
        category: def.category.clone(),
        inputs: canvas_port_templates(CanvasPortSide::Input, def),
        outputs: canvas_port_templates(CanvasPortSide::Output, def),
        params: canvas_node_param_templates(def),
    }
}

pub(crate) fn canvas_node_render_views(
    engine: &Engine,
    layouts: Vec<CanvasNodeLayout>,
    template_cache: &mut CanvasNodeTemplateCache,
) -> Vec<CanvasNodeRenderView> {
    let graph = engine.query_graph_snapshot();
    let defs_by_type = engine
        .list_node_defs()
        .into_iter()
        .map(|def| (def.type_id.as_str(), def))
        .collect::<HashMap<_, _>>();

    let mut views = layouts
        .into_iter()
        .filter_map(|layout| {
            let node_id = parse_engine_node_owner_id(&layout.owner_id)?;
            let node = graph.nodes.get(&node_id)?;
            let node_def = defs_by_type.get(node.type_id.as_str()).copied()?;
            let template = template_cache.template_for_def(node_def);
            let layout = layout_with_content_height(
                layout,
                template.params.len(),
                template.inputs.len(),
                template.outputs.len(),
            );
            let state = CanvasNodeInstanceState {
                owner_id: layout.owner_id.clone(),
                layout,
                selected: false,
                input_group: Default::default(),
                output_group: Default::default(),
                port_states: template
                    .inputs
                    .iter()
                    .chain(template.outputs.iter())
                    .map(|port| CanvasNodePortState {
                        key: port.key.clone(),
                        side: port.side,
                        connection_state: CanvasPortConnectionState::Idle,
                    })
                    .collect(),
            };
            Some(CanvasNodeRenderView { template, state })
        })
        .collect::<Vec<_>>();

    views.sort_by(|a, b| {
        a.state
            .layout
            .z_index
            .cmp(&b.state.layout.z_index)
            .then_with(|| a.state.owner_id.cmp(&b.state.owner_id))
    });
    views
}

pub(crate) fn canvas_connection_views(engine: &Engine) -> Vec<CanvasConnectionView> {
    let graph = engine.query_graph_snapshot();
    graph
        .connections
        .iter()
        .map(|connection| {
            let from_owner_id = engine_node_owner_id(connection.from.node);
            let to_owner_id = engine_node_owner_id(connection.to.node);
            CanvasConnectionView {
                from_port_id: canvas_port_stable_id(
                    &from_owner_id,
                    CanvasPortSide::Output,
                    &connection.from.interface,
                ),
                to_port_id: canvas_port_stable_id(
                    &to_owner_id,
                    CanvasPortSide::Input,
                    &connection.to.interface,
                ),
            }
        })
        .collect()
}

fn canvas_port_templates(
    side: CanvasPortSide,
    node_def: &engine::node_manager::NodeDef,
) -> Vec<CanvasNodePortTemplate> {
    exposed_ports(node_def, side)
        .into_iter()
        .map(|pin| CanvasNodePortTemplate::new(pin.name.clone(), pin.name, side))
        .collect()
}

fn exposed_ports(
    node_def: &engine::node_manager::NodeDef,
    side: CanvasPortSide,
) -> Vec<engine::node_manager::ExposedPinDef> {
    let mut ports = match side {
        CanvasPortSide::Input => node_def
            .inputs
            .iter()
            .map(|pin| engine::node_manager::ExposedPinDef {
                name: pin.name.clone(),
                data_type: pin.data_type.clone(),
                optional: pin.optional,
                kind: engine::node_manager::ExposedPinKind::Input,
                source: engine::node_manager::ExposedPinSource::Pin,
            })
            .collect::<Vec<_>>(),
        CanvasPortSide::Output => node_def
            .outputs
            .iter()
            .map(|pin| engine::node_manager::ExposedPinDef {
                name: pin.name.clone(),
                data_type: pin.data_type.clone(),
                optional: false,
                kind: engine::node_manager::ExposedPinKind::Output,
                source: engine::node_manager::ExposedPinSource::Pin,
            })
            .collect::<Vec<_>>(),
    };

    ports.extend(node_def.params.iter().filter_map(|param| {
        let exposed = match side {
            CanvasPortSide::Input => param
                .expose
                .contains(&engine::node_manager::ParamExpose::Input),
            CanvasPortSide::Output => param
                .expose
                .contains(&engine::node_manager::ParamExpose::Output),
        };
        exposed.then(|| engine::node_manager::ExposedPinDef {
            name: param.name.clone(),
            data_type: param.data_type.clone(),
            optional: side == CanvasPortSide::Input,
            kind: match side {
                CanvasPortSide::Input => engine::node_manager::ExposedPinKind::Input,
                CanvasPortSide::Output => engine::node_manager::ExposedPinKind::Output,
            },
            source: engine::node_manager::ExposedPinSource::Param,
        })
    }));
    ports
}

fn layout_with_content_height(
    mut layout: CanvasNodeLayout,
    param_count: usize,
    input_count: usize,
    output_count: usize,
) -> CanvasNodeLayout {
    let row_count = param_count.clamp(1, 5);
    let body_height = row_count as f32 * CANVAS_NODE_PARAM_ROW_HEIGHT
        + row_count.saturating_sub(1) as f32 * CANVAS_NODE_ROW_GAP_Y;
    let port_count = input_count.max(output_count);
    let port_height = if port_count == 0 {
        0.0
    } else {
        port_count as f32 * CANVAS_NODE_PIN_ROW_HEIGHT
            + port_count.saturating_sub(1) as f32 * CANVAS_NODE_PIN_ROW_GAP
    };
    layout.rect.h = layout
        .rect
        .h
        .max(CANVAS_NODE_MIN_HEIGHT)
        .max(CANVAS_NODE_PADDING_Y + body_height)
        .max(port_height);
    layout
}

fn canvas_node_param_templates(
    node_def: &engine::node_manager::NodeDef,
) -> Vec<CanvasNodeParamTemplate> {
    node_def
        .params
        .iter()
        .filter(|param| {
            param
                .expose
                .iter()
                .any(|expose| matches!(expose, engine::node_manager::ParamExpose::Control))
        })
        .take(5)
        .map(|param| {
            let value = compact_value(&param.default_value);
            CanvasNodeParamTemplate::new(
                param.name.clone(),
                param.name.clone(),
                param.data_type.to_string(),
                value,
                EngineControlSpecMap.control_for_param(param),
            )
        })
        .collect()
}

struct EngineControlSpecMap;

impl ControlSpecMap<engine::node_manager::ParamDef> for EngineControlSpecMap {
    fn control_for_param(&self, param: &engine::node_manager::ParamDef) -> ControlSpec {
        if let Some(constraint) = &param.constraint {
            match constraint.type_id.as_str() {
                "enum" => {
                    let options = constraint
                        .params
                        .get("options")
                        .and_then(|value| value.as_array())
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(|value| value.as_str().map(str::to_string))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let selected_value = match &param.default_value {
                        types::Value::String(value) => value.as_str(),
                        _ => "",
                    };
                    let selected = options
                        .iter()
                        .position(|option| option == selected_value)
                        .unwrap_or(0);
                    return ControlSpec::Select { options, selected };
                }
                "range" => {
                    let min = constraint
                        .params
                        .get("min")
                        .and_then(|value| value.as_f64())
                        .unwrap_or(0.0) as f32;
                    let max = constraint
                        .params
                        .get("max")
                        .and_then(|value| value.as_f64())
                        .unwrap_or(1.0) as f32;
                    return match &param.default_value {
                        types::Value::Float(value) => ControlSpec::Slider {
                            value: *value,
                            min,
                            max,
                            step: 0.01,
                        },
                        types::Value::Int(value) => ControlSpec::Number {
                            value: *value as f32,
                            min,
                            max,
                            step: 1.0,
                            precision: 0,
                        },
                        _ => ControlSpec::ReadOnly {
                            value: compact_value(&param.default_value),
                        },
                    };
                }
                "file_path" => {
                    let extensions = constraint
                        .params
                        .get("extensions")
                        .and_then(|value| value.as_array())
                        .map(|values| {
                            values
                                .iter()
                                .filter_map(|value| value.as_str().map(str::to_string))
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let path = match &param.default_value {
                        types::Value::String(value) => value.clone(),
                        _ => String::new(),
                    };
                    return ControlSpec::FilePath { path, extensions };
                }
                _ => {}
            }
        }

        match &param.default_value {
            types::Value::Float(value) => ControlSpec::Number {
                value: *value,
                min: f32::MIN,
                max: f32::MAX,
                step: 0.01,
                precision: 2,
            },
            types::Value::Int(value) => ControlSpec::Number {
                value: *value as f32,
                min: i32::MIN as f32,
                max: i32::MAX as f32,
                step: 1.0,
                precision: 0,
            },
            types::Value::Bool(value) => ControlSpec::Toggle { checked: *value },
            types::Value::Color(rgba) => ControlSpec::Color { rgba: *rgba },
            types::Value::String(value) => ControlSpec::Text {
                value: value.clone(),
            },
            types::Value::Image(_) | types::Value::Handle(_) => ControlSpec::ReadOnly {
                value: compact_value(&param.default_value),
            },
        }
    }
}

fn compact_value(value: &types::Value) -> String {
    match value {
        types::Value::Float(value) => format!("{value:.2}"),
        types::Value::Int(value) => value.to_string(),
        types::Value::Bool(value) => value.to_string(),
        types::Value::Color([r, g, b, _]) => format!("#{r:.1}{g:.1}{b:.1}"),
        types::Value::String(value) => {
            if value.is_empty() {
                "text".to_string()
            } else {
                value.chars().take(16).collect()
            }
        }
        types::Value::Image(_) => "image".to_string(),
        types::Value::Handle(handle) => handle.data_type.to_string(),
    }
}

fn default_canvas_node_rect(index: usize) -> Rect {
    let column = index % CANVAS_NODE_COLUMNS;
    let row = index / CANVAS_NODE_COLUMNS;
    Rect {
        x: column as f32 * CANVAS_NODE_COLUMN_GAP,
        y: row as f32 * CANVAS_NODE_ROW_GAP,
        w: CANVAS_NODE_WIDTH,
        h: CANVAS_NODE_MIN_HEIGHT,
    }
}

fn engine_node_owner_id(node_id: types::NodeId) -> String {
    format!("engine_node::{}", node_id.0)
}

pub(crate) fn parse_engine_node_owner_id(owner_id: &str) -> Option<types::NodeId> {
    let id = owner_id.strip_prefix("engine_node::")?.parse().ok()?;
    Some(types::NodeId(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::graph::{Connection, PinRef};
    use engine::node_manager::{
        ExecutionPolicy, NodeDef, NodeSourceKind, ParamDef, ParamExpose, PinDef, Purity,
    };

    #[test]
    fn engine_node_owner_id_roundtrips() {
        let owner_id = engine_node_owner_id(types::NodeId(42));

        assert_eq!(
            parse_engine_node_owner_id(&owner_id),
            Some(types::NodeId(42))
        );
        assert_eq!(parse_engine_node_owner_id("panel::42"), None);
    }

    #[test]
    fn canvas_connection_views_use_port_stable_ids() {
        let mut engine = Engine::new(None);
        let generator = engine.add_node("image_gen").unwrap();
        let color_adjust = engine.add_node("color_adjust").unwrap();
        engine
            .connect(Connection {
                from: PinRef {
                    node: generator,
                    interface: "image".to_string(),
                },
                to: PinRef {
                    node: color_adjust,
                    interface: "image".to_string(),
                },
            })
            .unwrap();

        let connections = canvas_connection_views(&engine);

        assert_eq!(connections.len(), 1);
        assert_eq!(
            connections[0].from_port_id,
            format!(
                "canvas_node::engine_node::{}::port::output::image",
                generator.0
            )
        );
        assert_eq!(
            connections[0].to_port_id,
            format!(
                "canvas_node::engine_node::{}::port::input::image",
                color_adjust.0
            )
        );
    }

    #[test]
    fn exposed_param_ports_are_included_in_canvas_ports() {
        let node_def = NodeDef {
            type_id: "test".to_string(),
            version: 1,
            source: NodeSourceKind::Builtin,
            name: "Test".to_string(),
            category: "test".to_string(),
            requires: Vec::new(),
            purity: Purity::Pure,
            cooking_sensitivity: Vec::new(),
            realtime_capable: true,
            execution: ExecutionPolicy::default(),
            api: None,
            inputs: vec![PinDef {
                name: "image".to_string(),
                data_type: types::DataType::image(),
                optional: false,
            }],
            outputs: vec![PinDef {
                name: "out".to_string(),
                data_type: types::DataType::image(),
                optional: false,
            }],
            params: vec![ParamDef {
                name: "strength".to_string(),
                data_type: types::DataType::float(),
                constraint: None,
                default_value: types::Value::Float(0.5),
                expose: vec![ParamExpose::Input],
            }],
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        };

        let input_ports = canvas_port_templates(CanvasPortSide::Input, &node_def);

        assert_eq!(input_ports.len(), 2);
        assert!(input_ports.iter().any(|port| port.key == "strength"
            && port.name == "strength"
            && port.side == CanvasPortSide::Input));
    }

    #[test]
    fn canvas_node_templates_hold_static_node_shape() {
        let engine = Engine::new(None);

        let templates = canvas_node_templates(&engine);
        let image_gen = templates.get("image_gen").expect("image_gen template");

        assert_eq!(image_gen.type_id, "image_gen");
        assert_eq!(image_gen.subtitle, "image_gen");
        assert!(image_gen.outputs.iter().any(|port| port.name == "image"));
        assert!(image_gen.params.iter().any(|param| param.key == "prompt"));
    }

    #[test]
    fn engine_control_spec_map_uses_value_shape() {
        let mapper = EngineControlSpecMap;

        assert!(matches!(
            mapper.control_for_param(&test_param(
                "enabled",
                types::DataType::bool(),
                types::Value::Bool(true),
                None,
            )),
            ControlSpec::Toggle { checked: true }
        ));
        assert!(matches!(
            mapper.control_for_param(&test_param(
                "prompt",
                types::DataType::string(),
                types::Value::String("hello".to_string()),
                None,
            )),
            ControlSpec::Text { value } if value == "hello"
        ));
        assert!(matches!(
            mapper.control_for_param(&test_param(
                "asset",
                types::DataType::handle(),
                types::Value::Handle(types::Handle::new(
                    "handle-1",
                    types::DataType::image(),
                    "test",
                    0,
                )),
                None,
            )),
            ControlSpec::ReadOnly { .. }
        ));
    }

    #[test]
    fn engine_control_spec_map_uses_constraints() {
        let mapper = EngineControlSpecMap;

        assert!(matches!(
            mapper.control_for_param(&test_param(
                "strength",
                types::DataType::float(),
                types::Value::Float(0.5),
                Some(types::Constraint::range(0.0, 1.0)),
            )),
            ControlSpec::Slider {
                value: 0.5,
                min: 0.0,
                max: 1.0,
                step: 0.01,
            }
        ));
        assert!(matches!(
            mapper.control_for_param(&test_param(
                "mode",
                types::DataType::string(),
                types::Value::String("B".to_string()),
                Some(types::Constraint::enum_options(vec![
                    "A".to_string(),
                    "B".to_string(),
                ])),
            )),
            ControlSpec::Select { options, selected }
                if options == vec!["A".to_string(), "B".to_string()] && selected == 1
        ));
        assert!(matches!(
            mapper.control_for_param(&test_param(
                "path",
                types::DataType::string(),
                types::Value::String("out.png".to_string()),
                Some(types::Constraint::file_path(vec!["png".to_string()])),
            )),
            ControlSpec::FilePath { path, extensions }
                if path == "out.png" && extensions == vec!["png".to_string()]
        ));
    }

    #[test]
    fn canvas_node_render_views_keep_instance_state_outside_template() {
        let mut engine = Engine::new(None);
        let node_id = engine.add_node("image_gen").unwrap();
        let owner_id = engine_node_owner_id(node_id);
        let mut template_cache = CanvasNodeTemplateCache::default();

        let views = canvas_node_render_views(
            &engine,
            vec![CanvasNodeLayout {
                owner_id: owner_id.clone(),
                rect: Rect {
                    x: 10.0,
                    y: 20.0,
                    w: CANVAS_NODE_WIDTH,
                    h: 40.0,
                },
                z_index: 7,
                collapsed: false,
                user_min_height: None,
            }],
            &mut template_cache,
        );

        assert_eq!(views.len(), 1);
        assert_eq!(views[0].template.type_id, "image_gen");
        assert_eq!(views[0].state.owner_id, owner_id);
        assert_eq!(views[0].state.layout.z_index, 7);
        assert!(!views[0].state.port_states.is_empty());
        assert!(views[0].state.layout.rect.h > 40.0);
        assert_eq!(template_cache.len(), 1);
    }

    #[test]
    fn template_cache_reuses_same_type_version() {
        let mut cache = CanvasNodeTemplateCache::default();
        let def = test_node_def("cache_test", 1);

        let first = cache.template_for_def(&def);
        let second = cache.template_for_def(&def);

        assert_eq!(first.type_id, "cache_test");
        assert_eq!(second.type_id, "cache_test");
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn template_cache_rebuilds_changed_version() {
        let mut cache = CanvasNodeTemplateCache::default();
        let first = test_node_def("cache_test", 1);
        let second = test_node_def("cache_test", 2);

        cache.template_for_def(&first);
        cache.template_for_def(&second);

        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn layout_height_grows_for_multiple_ports() {
        let layout = layout_with_content_height(
            CanvasNodeLayout {
                owner_id: "engine_node::1".to_string(),
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: CANVAS_NODE_WIDTH,
                    h: 40.0,
                },
                z_index: 0,
                collapsed: false,
                user_min_height: None,
            },
            1,
            5,
            1,
        );

        assert!(layout.rect.h > CANVAS_NODE_MIN_HEIGHT);
    }

    fn test_node_def(type_id: &str, version: u32) -> NodeDef {
        NodeDef {
            type_id: type_id.to_string(),
            version,
            source: NodeSourceKind::Builtin,
            name: "Cache Test".to_string(),
            category: "test".to_string(),
            requires: Vec::new(),
            purity: Purity::Pure,
            cooking_sensitivity: Vec::new(),
            realtime_capable: true,
            execution: ExecutionPolicy::default(),
            api: None,
            inputs: Vec::new(),
            outputs: Vec::new(),
            params: Vec::new(),
            execute: Box::new(|_ctx, inputs| Box::pin(async move { Ok(inputs) })),
        }
    }

    fn test_param(
        name: &str,
        data_type: types::DataType,
        default_value: types::Value,
        constraint: Option<types::Constraint>,
    ) -> ParamDef {
        ParamDef {
            name: name.to_string(),
            data_type,
            constraint,
            default_value,
            expose: vec![ParamExpose::Control],
        }
    }
}
