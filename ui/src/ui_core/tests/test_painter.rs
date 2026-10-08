#[test]
fn test_painter_records_all_commands() {
    use ui_core::paint::{BackdropMode, Color, Painter, Rect, TextureId, UvRect};
    let mut painter = Painter::new();
    painter.rect(Rect::new(1.0, 2.0, 10.0, 20.0), Color::new(1, 2, 3, 255));
    painter.rounded_rect(
        Rect::new(3.0, 4.0, 30.0, 40.0),
        5.0,
        Color::new(4, 5, 6, 255),
    );
    painter.text(7.0, 8.0, "row", Color::new(7, 8, 9, 255), 16.0, 1.5);
    painter.image(
        Rect::new(9.0, 10.0, 50.0, 60.0),
        TextureId::new(11),
        UvRect::full(),
        0.25,
        6.0,
    );
    painter.line((11.0, 12.0), (13.0, 14.0), 2.0, Color::new(10, 11, 12, 255));
    painter.circle((15.0, 16.0), 17.0, Color::new(13, 14, 15, 255));
    painter.path(
        &[(18.0, 19.0), (20.0, 21.0)],
        1.5,
        Color::new(16, 17, 18, 255),
        false,
    );
    painter.polygon(
        &[(25.0, 26.0), (27.0, 28.0), (29.0, 26.0)],
        Color::new(19, 20, 21, 255),
    );
    painter.backdrop(
        Rect::new(100.0, 200.0, 50.0, 60.0),
        BackdropMode::Blur(2.0),
        Color::new(236, 239, 244, 170),
    );
    let commands = painter.finish();
    assert_eq!(
        commands.len(),
        9,
        "expected 9 commands, got {}",
        commands.len()
    );
}
