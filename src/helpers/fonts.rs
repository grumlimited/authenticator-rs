use iced::Font;

pub const DEJAVU_SERIF: Font = Font::External {
    name: "Inconsolata-Bold",
    bytes: include_bytes!("../resources/fonts/DejaVuSerif.ttf"),
};

pub const INCONSOLATA_EXPANDED_BLACK: Font = Font::External {
    name: "Inconsolata-Bold",
    bytes: include_bytes!("../resources/fonts/Inconsolata-ExpandedBlack.ttf"),
};
