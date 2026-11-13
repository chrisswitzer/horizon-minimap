fn paper_screen(
    game: &crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
) {
    crate::game::paper_map_ui::screen(game,batch);
}
