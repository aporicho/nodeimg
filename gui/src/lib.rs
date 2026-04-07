pub mod canvas;
pub mod render;
pub mod state;
pub mod widgets;
pub mod panels;

use std::collections::HashSet;

use iced::widget::{canvas as iced_canvas, container, stack};
use iced::{Alignment, Element, Length, Task, Theme};

use engine::executors::image::GpuExecutor;
use engine::Engine;
use types::{NodeId, Vec2};

use canvas::{CanvasState, NodeCanvas, NodeCanvasData};
use canvas::controller::CanvasController;
use panels::toolbar::toolbar_view;
use panels::preview::preview_view;

pub struct App {
    engine: Engine,
    canvas_state: CanvasState,
    selection: HashSet<NodeId>,
    is_running: bool,
    preview_handle: Option<iced::widget::image::Handle>,
}

#[derive(Debug, Clone)]
pub enum Message {
    CanvasEvent(iced::widget::canvas::Event, f32, f32),
    AddNode(String),
    RunGraph,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        // GPU 初始化
        let gpu = pollster::block_on(Self::init_gpu());

        let engine = Engine::new(gpu);

        (
            App {
                engine,
                canvas_state: CanvasState::default(),
                selection: HashSet::new(),
                is_running: false,
                preview_handle: None,
            },
            Task::none(),
        )
    }

    async fn init_gpu() -> Option<GpuExecutor> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok()?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .ok()?;
        Some(GpuExecutor::new(device, queue))
    }

    pub fn title(&self) -> String {
        "nodeimg".to_string()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CanvasEvent(event, cx, cy) => {
                // 先拿 snapshot 避免借用冲突：
                // handle_event 需要 &Graph（只读）和 &mut GraphController（写）。
                // GraphController 拥有 Graph，不能同时 immutable + mutable borrow。
                // snapshot() 返回 Arc<Graph>，独立于 GraphController 的可变引用。
                let graph_snap = self.engine.graph.snapshot();
                let node_manager = &self.engine.node_manager;

                CanvasController::handle_event(
                    &event,
                    cx,
                    cy,
                    &mut self.canvas_state,
                    &graph_snap,
                    node_manager,
                    &mut self.selection,
                    &mut self.engine.graph,
                );
            }
            Message::AddNode(type_id) => {
                // 在视口中心附近放置新节点
                let center = self.canvas_state.viewport.to_canvas(Vec2::new(400.0, 300.0));
                match self.engine.graph.add_node(&type_id, center) {
                    Ok(id) => {
                        self.selection.clear();
                        self.selection.insert(id);
                    }
                    Err(e) => {
                        eprintln!("Failed to add node: {}", e);
                    }
                }
            }
            Message::RunGraph => {
                self.is_running = true;
                let result = pollster::block_on(self.engine.evaluate_all());
                self.is_running = false;

                match result {
                    Ok(results) => {
                        // 找到最后一个产出 Image 的节点，转为 CPU 并生成 iced Handle
                        if let Some(gpu) = self.engine.executor.gpu_executor() {
                            'outer: for outputs in results.values() {
                                for value in outputs.values() {
                                    if let types::Value::Image(img) = value {
                                        let cpu = img.as_cpu(&gpu.device, &gpu.queue);
                                        let rgba = cpu.to_rgba8();
                                        let (w, h) = rgba.dimensions();
                                        self.preview_handle = Some(
                                            iced::widget::image::Handle::from_rgba(w, h, rgba.into_raw())
                                        );
                                        break 'outer;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Execution error: {}", e);
                    }
                }
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let graph = self.engine.graph.current();
        let node_manager = &self.engine.node_manager;

        // 画布数据
        let canvas_data = NodeCanvasData {
            graph,
            node_manager,
            selection: &self.selection,
            canvas_state: &self.canvas_state,
        };

        // 画布 widget（填满整个窗口）
        let canvas_widget: Element<'_, Message> = iced_canvas(NodeCanvas::new(canvas_data))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        // 工具栏
        let node_types: Vec<(&str, &str)> = vec![
            ("load_image", "LoadImage"),
            ("brightness", "Brightness"),
        ];
        let toolbar = toolbar_view(&node_types, self.is_running);

        // 工具栏居中浮动在顶部
        let toolbar_layer: Element<'_, Message> = container(toolbar)
            .width(Length::Fill)
            .center_x(Length::Fill)
            .padding(8)
            .into();

        // 预览面板浮动在左侧
        let preview = preview_view(self.preview_handle.as_ref());
        let preview_layer: Element<'_, Message> = container(preview)
            .width(Length::Shrink)
            .height(Length::Fill)
            .align_x(Alignment::Start)
            .into();

        // stack：画布在底层，预览面板和工具栏浮动在上层
        stack![canvas_widget, preview_layer, toolbar_layer].into()
    }

    pub fn theme(&self) -> Theme {
        Theme::Light
    }
}
