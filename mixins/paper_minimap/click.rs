fn paper_click(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    button: usize,
) {
    use crate::game::paper_minimap as m;
    if mod_id!=m::ID{return;}
    m::with_state(|s|{
        let old=s.config;
        if let Some((key,value))=s.config.cycle(button) {
            let result=m::atomic_write(&game.native_data_dir(m::ID).join(key),value.as_bytes());
            if result.is_err(){s.config=old;}
            s.notice(result,"Setting saved");
            s.image_dirty=true;
        }
    });
}
