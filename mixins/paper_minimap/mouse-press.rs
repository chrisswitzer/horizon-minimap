fn paper_mouse_press(
    game: &crate::game::Game<'_>,
    button: i32,
    canceled: &mut bool,
) {
    if game.native_mod_screen()!=Some(crate::game::paper_minimap::ID)||!game.native_on(crate::game::paper_minimap::ID){return;}
    if *canceled{return;}
    *canceled=true;
    crate::game::paper_map_ui::click(game,button);
}
