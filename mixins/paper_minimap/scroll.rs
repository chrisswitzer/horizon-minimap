fn paper_scroll(
    game: &crate::game::Game<'_>,
    scroll: [f64; 2],
    canceled: &mut bool,
) {
    if game.native_mod_screen()!=Some(crate::game::paper_minimap::ID)||!game.native_on(crate::game::paper_minimap::ID){return;}
    if *canceled{return;}
    *canceled=true;
    crate::game::paper_map_ui::scroll(game,scroll[1]);
}
