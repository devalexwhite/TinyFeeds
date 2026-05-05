use iced::{Theme, border, widget::button};

pub fn button_outline(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();

    let base = button::Style {
        background: Some(palette.background.base.color.into()),
        border: border::rounded(10)
            .color(palette.background.base.text)
            .width(1),
        ..button::Style::default()
    };

    match status {
        button::Status::Active | button::Status::Pressed => base,
        button::Status::Hovered => button::Style {
            background: Some(palette.primary.base.color.into()),
            text_color: palette.primary.base.text,
            ..base
        },
        button::Status::Disabled => button::Style {
            text_color: palette.danger.base.color.scale_alpha(0.5),
            ..base
        },
    }
    //
}
