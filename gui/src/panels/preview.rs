use iced::widget::{column, container, image, text};
use iced::{Element, Length};
use crate::Message;

pub fn preview_view<'a>(
    preview_image: Option<&'a iced::widget::image::Handle>,
) -> Element<'a, Message> {
    let content: Element<'a, Message> = match preview_image {
        Some(handle) => {
            image(handle.clone())
                .width(Length::Fill)
                .into()
        }
        None => {
            text("No preview")
                .size(12)
                .into()
        }
    };

    container(
        column![
            container(text("Preview").size(13)).padding([10, 14]),
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(12),
        ]
    )
    .width(240)
    .height(Length::Fill)
    .into()
}
