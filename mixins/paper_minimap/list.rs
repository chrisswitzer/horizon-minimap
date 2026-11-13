fn paper_list(
    mods: &mut Vec<crate::game::native_hooks::NativeMod>,
) {
    use crate::game::paper_minimap as m;
    let default=m::with_state(|s|s.hotkey.clone());
    mods.push(crate::game::native_hooks::NativeMod {
        id:m::ID.into(), title:format!("Paper Minimap {}",m::VERSION),
        description:"Explored paper maps, full-screen atlas and per-world waypoints.".into(),
        keys:vec![["full_map".into(),"Full map (M; J if M conflicts)".into(),default]],
    });
}
