fn paper_mouse_release(
    game: &crate::game::Game<'_>,
    canceled: &mut bool,
) {
    if game.native_mod_screen()!=Some(crate::game::paper_minimap::ID)||!game.native_on(crate::game::paper_minimap::ID){return;}
    if *canceled{return;}
    *canceled=true;
    crate::game::paper_minimap::with_state(|s|s.dragging=false);
}
