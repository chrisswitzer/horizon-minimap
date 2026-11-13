fn paper_character(
    game: &crate::game::Game<'_>,
    c: char,
    canceled: &mut bool,
) {
    if game.native_mod_screen()!=Some(crate::game::paper_minimap::ID)||!game.native_on(crate::game::paper_minimap::ID){return;}
    if *canceled{return;}
    *canceled=true;
    use crate::game::paper_minimap as m;
    m::with_state(|s|{if s.page==m::Page::Editor{if let Some(e)=s.editor.as_mut(){e.character(c);}}});
}
