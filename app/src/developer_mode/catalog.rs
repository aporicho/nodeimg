#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum PlaygroundItemId {
    Canvas,
    PrimitiveConcept,
    ContainerConcept,
    ControlConcept,
    OverlayConcept,
    WorkspaceConcept,
    Surface,
    Text,
    Image,
    Icon,
    Circle,
    Line,
    Curve,
    Path,
    Grid,
    Connection,
    ContainerBox,
    RowLayout,
    ColumnLayout,
    ScrollArea,
    Group,
    Collapsible,
    Label,
    Button,
    TextInput,
    Slider,
    NumberInput,
    Toggle,
    Checkbox,
    Radio,
    Dropdown,
    ImageViewer,
    Popup,
    Panel,
    NodeCard,
}

impl PlaygroundItemId {
    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Canvas => "canvas",
            Self::PrimitiveConcept => "primitive_concept",
            Self::ContainerConcept => "container_concept",
            Self::ControlConcept => "control_concept",
            Self::OverlayConcept => "overlay_concept",
            Self::WorkspaceConcept => "workspace_concept",
            Self::Surface => "surface",
            Self::Text => "text",
            Self::Image => "image",
            Self::Icon => "icon",
            Self::Circle => "circle",
            Self::Line => "line",
            Self::Curve => "curve",
            Self::Path => "path",
            Self::Grid => "grid",
            Self::Connection => "connection",
            Self::ContainerBox => "container_box",
            Self::RowLayout => "row_layout",
            Self::ColumnLayout => "column_layout",
            Self::ScrollArea => "scroll_area",
            Self::Group => "group",
            Self::Collapsible => "collapsible",
            Self::Label => "label",
            Self::Button => "button",
            Self::TextInput => "text_input",
            Self::Slider => "slider",
            Self::NumberInput => "number_input",
            Self::Toggle => "toggle",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Dropdown => "dropdown",
            Self::ImageViewer => "image_viewer",
            Self::Popup => "popup",
            Self::Panel => "panel",
            Self::NodeCard => "node_card",
        }
    }

    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Some(match key {
            "canvas" => Self::Canvas,
            "primitive_concept" => Self::PrimitiveConcept,
            "container_concept" => Self::ContainerConcept,
            "control_concept" => Self::ControlConcept,
            "overlay_concept" => Self::OverlayConcept,
            "workspace_concept" => Self::WorkspaceConcept,
            "surface" => Self::Surface,
            "text" => Self::Text,
            "image" => Self::Image,
            "icon" => Self::Icon,
            "circle" => Self::Circle,
            "line" => Self::Line,
            "curve" => Self::Curve,
            "path" => Self::Path,
            "grid" => Self::Grid,
            "connection" => Self::Connection,
            "container_box" => Self::ContainerBox,
            "row_layout" => Self::RowLayout,
            "column_layout" => Self::ColumnLayout,
            "scroll_area" => Self::ScrollArea,
            "group" => Self::Group,
            "collapsible" => Self::Collapsible,
            "label" => Self::Label,
            "button" => Self::Button,
            "text_input" => Self::TextInput,
            "slider" => Self::Slider,
            "number_input" => Self::NumberInput,
            "toggle" => Self::Toggle,
            "checkbox" => Self::Checkbox,
            "radio" => Self::Radio,
            "dropdown" => Self::Dropdown,
            "image_viewer" => Self::ImageViewer,
            "popup" => Self::Popup,
            "panel" => Self::Panel,
            "node_card" => Self::NodeCard,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlaygroundGroup {
    Foundation,
    Family,
    Primitive,
    Container,
    Control,
    Overlay,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlaygroundItemSpec {
    pub(crate) id: PlaygroundItemId,
    pub(crate) group: PlaygroundGroup,
    pub(crate) title: &'static str,
    pub(crate) description: &'static str,
    pub(crate) size: [f32; 2],
    pub(crate) default_position: [f32; 2],
}

const CARD_W: f32 = 190.0;
const CARD_H: f32 = 108.0;
const FAMILY_W: f32 = 210.0;
const FOUNDATION_W: f32 = 300.0;

const ITEM_SPECS: &[PlaygroundItemSpec] = &[
    spec(
        PlaygroundItemId::Canvas,
        PlaygroundGroup::Foundation,
        "Canvas",
        "Root drawing and interaction space",
        [FOUNDATION_W, CARD_H],
        [24.0, 24.0],
    ),
    spec(
        PlaygroundItemId::PrimitiveConcept,
        PlaygroundGroup::Family,
        "Primitives",
        "Smallest renderable drawing concepts",
        [FAMILY_W, CARD_H],
        [24.0, 160.0],
    ),
    spec(
        PlaygroundItemId::ContainerConcept,
        PlaygroundGroup::Family,
        "Containers",
        "Layout, grouping, clipping and scrolling",
        [FAMILY_W, CARD_H],
        [264.0, 160.0],
    ),
    spec(
        PlaygroundItemId::ControlConcept,
        PlaygroundGroup::Family,
        "Controls",
        "Interactive value and command widgets",
        [FAMILY_W, CARD_H],
        [504.0, 160.0],
    ),
    spec(
        PlaygroundItemId::OverlayConcept,
        PlaygroundGroup::Family,
        "Overlay",
        "Floating UI anchored to another concept",
        [FAMILY_W, CARD_H],
        [744.0, 160.0],
    ),
    spec(
        PlaygroundItemId::WorkspaceConcept,
        PlaygroundGroup::Family,
        "Workspace",
        "Panels, canvas nodes and graph editing",
        [FAMILY_W, CARD_H],
        [984.0, 160.0],
    ),
    primitive(
        PlaygroundItemId::Surface,
        "Surface",
        "Fill, border, radius, shadow",
        24.0,
        306.0,
    ),
    primitive(
        PlaygroundItemId::Text,
        "Text",
        "Clip, align, ellipsis",
        234.0,
        306.0,
    ),
    primitive(
        PlaygroundItemId::Image,
        "Image",
        "Fit, filter, tint",
        444.0,
        306.0,
    ),
    primitive(
        PlaygroundItemId::Icon,
        "Icon",
        "SVG icon leaf",
        654.0,
        306.0,
    ),
    primitive(
        PlaygroundItemId::Circle,
        "Circle",
        "Fill and stroke",
        864.0,
        306.0,
    ),
    primitive(
        PlaygroundItemId::Line,
        "Line",
        "Cap and stroke width",
        24.0,
        430.0,
    ),
    primitive(
        PlaygroundItemId::Curve,
        "Curve",
        "Cubic Bezier stroke",
        234.0,
        430.0,
    ),
    primitive(
        PlaygroundItemId::Path,
        "Path",
        "Even-odd fill path",
        444.0,
        430.0,
    ),
    primitive(
        PlaygroundItemId::Grid,
        "Grid",
        "Canvas grid primitive",
        654.0,
        430.0,
    ),
    primitive(
        PlaygroundItemId::Connection,
        "Connection",
        "Port-to-port curve",
        864.0,
        430.0,
    ),
    container(
        PlaygroundItemId::ContainerBox,
        "Container",
        "Box model and decoration",
        24.0,
        574.0,
    ),
    container(
        PlaygroundItemId::RowLayout,
        "Row",
        "Horizontal layout relation",
        234.0,
        574.0,
    ),
    container(
        PlaygroundItemId::ColumnLayout,
        "Column",
        "Vertical layout relation",
        444.0,
        574.0,
    ),
    container(
        PlaygroundItemId::ScrollArea,
        "Scroll area",
        "Overflow and scroll state",
        654.0,
        574.0,
    ),
    container(
        PlaygroundItemId::Group,
        "Group",
        "Titled grouped content",
        864.0,
        574.0,
    ),
    container(
        PlaygroundItemId::Collapsible,
        "Collapsible",
        "Expandable container",
        1074.0,
        574.0,
    ),
    control(
        PlaygroundItemId::Label,
        "Label",
        "Semantic text widget",
        24.0,
        718.0,
    ),
    control(
        PlaygroundItemId::Button,
        "Button",
        "Tap action state",
        234.0,
        718.0,
    ),
    control(
        PlaygroundItemId::TextInput,
        "Text input",
        "Editable text field",
        444.0,
        718.0,
    ),
    control(
        PlaygroundItemId::Slider,
        "Slider",
        "Drag value control",
        654.0,
        718.0,
    ),
    control(
        PlaygroundItemId::NumberInput,
        "Number input",
        "Formatted numeric value",
        864.0,
        718.0,
    ),
    control(
        PlaygroundItemId::Toggle,
        "Toggle",
        "Binary switch state",
        24.0,
        842.0,
    ),
    control(
        PlaygroundItemId::Checkbox,
        "Checkbox",
        "Checked state",
        234.0,
        842.0,
    ),
    control(
        PlaygroundItemId::Radio,
        "Radio",
        "Exclusive choice",
        444.0,
        842.0,
    ),
    control(
        PlaygroundItemId::Dropdown,
        "Dropdown",
        "Overlay option list",
        654.0,
        842.0,
    ),
    control(
        PlaygroundItemId::ImageViewer,
        "Image viewer",
        "Texture preview control",
        864.0,
        842.0,
    ),
    spec(
        PlaygroundItemId::Popup,
        PlaygroundGroup::Overlay,
        "Popup",
        "Anchored overlay trigger",
        [CARD_W, CARD_H],
        [744.0, 984.0],
    ),
    spec(
        PlaygroundItemId::Panel,
        PlaygroundGroup::Workspace,
        "Panel",
        "Floating workspace surface",
        [CARD_W, CARD_H],
        [984.0, 984.0],
    ),
    spec(
        PlaygroundItemId::NodeCard,
        PlaygroundGroup::Workspace,
        "Node card",
        "Graph node visual structure",
        [CARD_W, CARD_H],
        [24.0, 1108.0],
    ),
];

pub(crate) fn item_specs() -> &'static [PlaygroundItemSpec] {
    ITEM_SPECS
}

#[cfg(test)]
pub(crate) fn spec_for(id: PlaygroundItemId) -> Option<&'static PlaygroundItemSpec> {
    ITEM_SPECS.iter().find(|spec| spec.id == id)
}

const fn primitive(
    id: PlaygroundItemId,
    title: &'static str,
    description: &'static str,
    x: f32,
    y: f32,
) -> PlaygroundItemSpec {
    spec(
        id,
        PlaygroundGroup::Primitive,
        title,
        description,
        [CARD_W, CARD_H],
        [x, y],
    )
}

const fn container(
    id: PlaygroundItemId,
    title: &'static str,
    description: &'static str,
    x: f32,
    y: f32,
) -> PlaygroundItemSpec {
    spec(
        id,
        PlaygroundGroup::Container,
        title,
        description,
        [CARD_W, CARD_H],
        [x, y],
    )
}

const fn control(
    id: PlaygroundItemId,
    title: &'static str,
    description: &'static str,
    x: f32,
    y: f32,
) -> PlaygroundItemSpec {
    spec(
        id,
        PlaygroundGroup::Control,
        title,
        description,
        [CARD_W, CARD_H],
        [x, y],
    )
}

const fn spec(
    id: PlaygroundItemId,
    group: PlaygroundGroup,
    title: &'static str,
    description: &'static str,
    size: [f32; 2],
    default_position: [f32; 2],
) -> PlaygroundItemSpec {
    PlaygroundItemSpec {
        id,
        group,
        title,
        description,
        size,
        default_position,
    }
}
