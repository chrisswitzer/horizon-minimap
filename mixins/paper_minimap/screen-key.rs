fn paper_screen_key(
    game: &crate::game::Game<'_>,
    key: &str,
    repeat: bool,
    modifiers: u32,
    canceled: &mut bool,
) {
    if game.native_mod_screen()!=Some(crate::game::paper_minimap::ID)||!game.native_on(crate::game::paper_minimap::ID){return;}
    if *canceled{return;}
    *canceled=true;
    crate::game::paper_map_ui::screen_key(key,repeat,modifiers);
}
