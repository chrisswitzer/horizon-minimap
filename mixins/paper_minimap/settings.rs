fn paper_settings(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    buttons: &mut Vec<String>,
) {
    use crate::game::paper_minimap as m;
    if mod_id!=m::ID{return;}
    let config=m::Settings::read(|key|game.native_setting(m::ID,key));
    m::with_state(|s|{s.config=config;s.config_loaded=true;s.image_dirty=true;});
    buttons.extend((0..m::SPEC_COUNT).map(|i|config.label(i)));
}
