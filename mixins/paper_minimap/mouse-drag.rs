fn paper_mouse_drag(
    game: &crate::game::Game<'_>,
    button: i32,
    delta: [f32; 2],
    canceled: &mut bool,
) {
    if game.native_mod_screen()!=Some(crate::game::paper_minimap::ID)||!game.native_on(crate::game::paper_minimap::ID){return;}
    if *canceled{return;}
    *canceled=true;
    use crate::game::paper_minimap as m;
    m::with_state(|s|{if button==0&&s.dragging&&s.page==m::Page::Map{s.view.pan(delta);s.image_dirty=true;}});
}
