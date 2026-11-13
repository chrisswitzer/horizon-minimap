fn paper_key(
    game: &mut crate::game::Game<'_>,
    key: &str,
    pressed: bool,
) {
    crate::game::paper_map_ui::key(game,key,pressed);
}
