use iced::widget::{button, container, row, text};
use iced::Element;
use crate::Message;

pub fn toolbar_view<'a>(node_types: &[(&str, &str)], is_running: bool) -> Element<'a, Message> {
    let mut buttons = row![].spacing(4);

    for (type_id, label) in node_types {
        buttons = buttons.push(
            button(text(format!("+ {}", label)).size(12))
                .on_press(Message::AddNode(type_id.to_string()))
                .padding([6, 12]),
        );
    }

    let run_btn = if is_running {
        button(text("Running...").size(12)).padding([8, 24])
    } else {
        button(text("▶ Run").size(12))
            .on_press(Message::RunGraph)
            .padding([8, 24])
    };

    buttons = buttons.push(run_btn);

    container(buttons)
        .padding(4)
        .into()
}
