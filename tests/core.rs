use paper_minimap::*;
use std::{fs, path::PathBuf, sync::atomic::{AtomicU64, Ordering}};
static SERIAL: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let unique = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("paper-map-test-{}-{unique}-{}", std::process::id(), SERIAL.fetch_add(1, Ordering::Relaxed)));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {fn drop(&mut self) {let _ = fs::remove_dir_all(&self.0);}}
fn pixel() -> Pixel {Pixel {rgb:0xff73a33d,height:64}}
fn point() -> Waypoint {Waypoint {id:1,name:"Home 森".into(),pos:[-32.5,72.0,400.25],color:COLORS[2],icon:1,enabled:true,in_world:false}}
#[test]
fn settings_reject_invalid_values_and_preserve_legacy_corner() {
    let s=Settings::read(|k|match k {"size"=>Some("999999".into()),"corner"=>Some("bottom_left".into()),"world_markers"=>Some("invalid".into()),_=>None});
    assert_eq!(s.0[MINI_SIZE],128);assert_eq!(s.0[MINI_CORNER],2);assert!(!s.on(WORLD_MARKERS));
    let mut s=s;assert_eq!(s.cycle(SPEC_COUNT),None);
}
#[test]
fn automatic_hotkey_falls_back_without_overwriting_user_binding() {
    let keys=vec!["key.keyboard.m".into()];
    assert_eq!(choose_hotkey(None,&[]),("key.keyboard.m".into(),false));
    assert_eq!(choose_hotkey(None,&keys),("key.keyboard.j".into(),false));
    assert_eq!(choose_hotkey(Some("key.keyboard.k"),&keys),("key.keyboard.k".into(),false));
    assert_eq!(choose_hotkey(Some("key.keyboard.m"),&keys),("key.keyboard.m".into(),true));
    assert!(choose_hotkey(None,&["key.keyboard.m".into(),"key.keyboard.j".into()]).1);
}
#[test]
fn zoom_keeps_cursor_world_position_and_has_finite_limits() {
    let rect=Rect{x:12.0,y:38.0,w:800.0,h:500.0};
    let cursor=[210.0,330.0];
    let mut view=View{center:[-1256.25,4000.5],bpp:2.0};
    let before=view.world(rect,cursor);
    view.zoom(rect,cursor,2.0);
    let after=view.world(rect,cursor);
    assert!((before[0]-after[0]).abs()<1e-9 && (before[1]-after[1]).abs()<1e-9);
    let screen=view.screen(rect,before);
    assert!((screen[0]-cursor[0]).abs()<0.001 && (screen[1]-cursor[1]).abs()<0.001);
    for _ in 0..50 {view.zoom(rect,cursor,4.0);}
    assert_eq!(view.bpp,0.25);
    for _ in 0..50 {view.zoom(rect,cursor,-4.0);}
    assert_eq!(view.bpp,32.0);
    let saved=view;view.zoom(rect,cursor,f64::NAN);assert_eq!(view,saved);
    view.pan([f32::NAN,f32::INFINITY]);assert_eq!(view,saved);
}
#[test]
fn exploration_roundtrips_negative_coordinates_without_revealing_neighbors() {
    let t=Temp::new();
    let mut store=Store::open(&t.0,"world-a","overworld").unwrap();
    for x in -1000..1000 {assert_eq!(store.get(x,8000),None);}
    assert!(store.tiles.is_empty(),"blank panning must not allocate terrain");
    for (x,z) in [(-1,-1),(-32,-33),(0,0),(32,31)] {store.set(x,z,pixel());}
    store.flush().unwrap();drop(store);
    let mut store=Store::open(&t.0,"world-a","overworld").unwrap();
    for (x,z) in [(-1,-1),(-32,-33),(0,0),(32,31)] {assert_eq!(store.get(x,z),Some(pixel()));}
    assert_eq!(store.get(-2,-1),None);assert_eq!(store.get(10000,10000),None);
}
#[test]
fn worlds_and_dimensions_do_not_share_exploration_or_waypoints() {
    let t=Temp::new();
    let mut a=Store::open(&t.0,"world-a","overworld").unwrap();
    a.set(0,0,pixel());a.save_waypoint(point()).unwrap();a.flush().unwrap();
    for (world,dim) in [("world-b","overworld"),("world-a","nether")] {
        let mut b=Store::open(&t.0,world,dim).unwrap();
        assert_ne!(a.root,b.root);assert_eq!(b.get(0,0),None);assert!(b.waypoints.is_empty());
    }
}
#[test]
fn waypoint_edit_delete_and_unicode_roundtrip() {
    let t=Temp::new();
    let mut s=Store::open(&t.0,"world","overworld").unwrap();
    let mut p=point();s.save_waypoint(p.clone()).unwrap();
    p.name="Base 雪".into();p.in_world=true;s.save_waypoint(p.clone()).unwrap();
    drop(s);
    let mut s=Store::open(&t.0,"world","overworld").unwrap();
    assert_eq!(s.waypoints,vec![p]);s.remove_waypoint(1).unwrap();drop(s);
    assert!(Store::open(&t.0,"world","overworld").unwrap().waypoints.is_empty());
}
#[test]
fn invalid_waypoints_and_duplicate_ids_are_rejected() {
    let mut p=point();p.pos[0]=f64::NAN;assert!(!p.valid());
    p=point();p.name="bad\nname".into();assert!(!p.valid());
    let bytes=encode_waypoints(&[point(),point()]);assert!(decode_waypoints(&bytes).is_err());
    assert!(Tile::decode(b"PMT2").is_err());
}
#[test]
fn corrupted_files_are_preserved_and_never_silently_overwritten() {
    let t=Temp::new();
    let s=Store::open(&t.0,"world","overworld").unwrap();let root=s.root.clone();drop(s);
    fs::write(root.join("0_0.pmt"),b"broken tile").unwrap();
    fs::write(root.join("waypoints.txt"),b"broken waypoint").unwrap();
    let mut s=Store::open(&t.0,"world","overworld").unwrap();
    assert_eq!(s.get(0,0),None);s.set(0,0,pixel());s.flush().unwrap();
    assert!(s.save_waypoint(point()).is_err());
    assert_eq!(fs::read(root.join("0_0.pmt")).unwrap(),b"broken tile");
    assert_eq!(fs::read(root.join("waypoints.txt")).unwrap(),b"broken waypoint");
}
#[test]
fn failed_saves_retain_dirty_tiles_and_existing_waypoints() {
    let t=Temp::new();
    let mut s=Store::open(&t.0,"world","overworld").unwrap();
    s.save_waypoint(point()).unwrap();
    fs::create_dir(s.root.join("waypoints.pending")).unwrap();
    let mut changed=point();changed.name="Changed".into();
    assert!(s.save_waypoint(changed).is_err());assert_eq!(s.waypoints,vec![point()]);
    s.set(0,0,pixel());
    fs::create_dir(s.root.join("0_0.pending")).unwrap();
    assert!(s.flush().is_err());assert!(s.tiles[&(0,0)].dirty);
    fs::remove_dir(s.root.join("0_0.pending")).unwrap();
    s.flush().unwrap();assert!(!s.tiles[&(0,0)].dirty);
}
#[test]
fn exploration_is_limited_to_the_current_player_radius() {
    let offsets=reveal_offsets(16);
    assert_eq!(offsets[0],[0,0]);
    assert!(offsets.iter().all(|p|p[0]*p[0]+p[1]*p[1]<=256));
    assert!(in_reveal_radius([0,0],[0.5,64.0,0.5],16));
    assert!(!in_reveal_radius([0,0],[1000.0,64.0,1000.0],16));
}
#[test]
fn new_waypoints_default_to_map_only_and_world_switch_discards_editor() {
    let t=Temp::new();let mut s=State::default();s.player=[12.0,72.0,34.0];
    assert!(s.switch_world(&t.0,"world","overworld"));s.begin_waypoint([12.0,34.0]);
    let p=s.editor.as_ref().unwrap().point().unwrap();
    assert!(p.enabled);assert!(!p.in_world);assert!(!s.config.on(WORLD_MARKERS));
    s.actions.push(Action::Save);
    assert!(s.switch_world(&t.0,"other","overworld"));
    assert!(s.editor.is_none());assert!(s.actions.is_empty());assert_eq!(s.page,Page::Map);
}
#[test]
fn cache_eviction_saves_explored_tiles_and_keeps_memory_bounded() {
    let t=Temp::new();let mut s=Store::open(&t.0,"world","overworld").unwrap();
    for i in 0..CACHE_TILES+2 {s.set(i as i32*TILE_SIDE,0,pixel());}
    assert!(s.tiles.len()<=CACHE_TILES);
    assert_eq!(s.get(0,0),Some(pixel()));
    s.flush().unwrap();drop(s);
    let mut s=Store::open(&t.0,"world","overworld").unwrap();
    assert_eq!(s.get((CACHE_TILES+1) as i32*TILE_SIDE,0),Some(pixel()));
}
