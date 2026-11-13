fn paper_world_geometry(
    game: &mut crate::game::Game<'_>,
    camera: glam::DVec3,
    partial: f32,
    out: &mut Vec<horizon_client::mesher::Vertex>,
) {
    use crate::game::paper_minimap as m;
    if !game.native_on(m::ID)||!m::with_state(|s|s.config.on(m::WORLD_MARKERS)){return;}
    // This entry point runs before Horizon's no-typed-mods early-out.
    // A native_world_submit hook alone would not activate world geometry.
    let mut draw=NativeWorldDraw::new(camera.to_array(),partial,game.world.game_time);
    game.mark_stage(out.len(),"NONE");
    crate::game::paper_map_ui::world_markers(game,&mut draw);
    game.resolve_inline_profiles(&draw.profiles);
    game.map_patches.extend(draw.patches);
    out.extend(draw.draw.into_vertices());
}
