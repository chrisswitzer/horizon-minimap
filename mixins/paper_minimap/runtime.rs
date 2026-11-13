// Paper Minimap core: no Horizon dependencies, so cargo test runs without the game.
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const ID: &str = "paper_minimap";
pub const VERSION: &str = "0.2.0";
pub const MAX_WAYPOINTS: usize = 256;
pub const COORD_LIMIT: f64 = 30_000_000.0;
pub const TILE_SIDE: i32 = 32;
pub const TILE_CELLS: usize = 1024;
pub const CACHE_TILES: usize = 1024;
pub const MINI_SHOW: usize = 0;
pub const MINI_SIZE: usize = 1;
pub const MINI_CORNER: usize = 2;
pub const MINI_ZOOM: usize = 3;
pub const GUI_SCALE: usize = 4;
pub const BORDER: usize = 5;
pub const MAP_ALPHA: usize = 6;
pub const MARKER_SIZE: usize = 7;
pub const LABELS: usize = 8;
pub const DISTANCES: usize = 9;
pub const LABEL_BG: usize = 10;
pub const WORLD_MARKERS: usize = 11;
pub const WORLD_RANGE: usize = 12;
pub const WORLD_SCALE: usize = 13;
pub const WORLD_ALPHA: usize = 14;
pub const WORLD_HEIGHT: usize = 15;
pub const REVEAL_RADIUS: usize = 16;
pub const SAMPLE_BUDGET: usize = 17;
pub const SAVE_SECONDS: usize = 18;
pub const FOLLOW: usize = 19;
pub const NORTH: usize = 20;
pub const MARKER_ALPHA: usize = 21;
pub const SPEC_COUNT: usize = 22;

pub struct Spec {
    pub key: &'static str,
    pub label: &'static str,
    pub values: &'static [i32],
    pub default: i32,
}
pub const SPECS: [Spec; SPEC_COUNT] = [
    Spec { key:"minimap", label:"Show minimap", values:&[0,1], default:1 },
    Spec { key:"size", label:"Minimap size (GUI px)", values:&[64,96,128,160,192,256], default:128 },
    Spec { key:"corner", label:"Minimap corner", values:&[0,1,2,3], default:1 },
    Spec { key:"zoom", label:"Minimap blocks/pixel", values:&[1,2,4,8,16], default:1 },
    Spec { key:"gui_scale", label:"Map controls / text size (%)", values:&[75,100,125,150,200], default:100 },
    Spec { key:"border", label:"Paper border width (px)", values:&[0,3,5,7,10], default:7 },
    Spec { key:"map_alpha", label:"Minimap opacity (%)", values:&[40,60,80,100], default:100 },
    Spec { key:"marker_size", label:"Map waypoint size (px)", values:&[4,6,8,10,12,16], default:8 },
    Spec { key:"labels", label:"Waypoint names on map", values:&[0,1], default:1 },
    Spec { key:"distances", label:"Waypoint distances", values:&[0,1], default:1 },
    Spec { key:"label_bg", label:"Waypoint label backgrounds", values:&[0,1], default:1 },
    Spec { key:"world_markers", label:"Show waypoints in world", values:&[0,1], default:0 },
    Spec { key:"world_range", label:"World marker range (blocks)", values:&[64,128,256,512,1024,2048], default:512 },
    Spec { key:"world_scale", label:"World marker / label size (%)", values:&[50,75,100,150,200], default:100 },
    Spec { key:"world_alpha", label:"World marker opacity (%)", values:&[25,50,75,100], default:75 },
    Spec { key:"world_height", label:"World marker height offset", values:&[1,2,4,8,16], default:2 },
    Spec { key:"reveal_radius", label:"Exploration radius (blocks)", values:&[16,24,32,48,64], default:32 },
    Spec { key:"sample_budget", label:"Terrain columns per tick", values:&[32,64,128,256,512], default:128 },
    Spec { key:"save_seconds", label:"Exploration autosave (sec)", values:&[5,10,30,60], default:10 },
    Spec { key:"follow", label:"Follow player when map opens", values:&[0,1], default:1 },
    Spec { key:"north", label:"Show north indicator", values:&[0,1], default:1 },
    Spec { key:"marker_alpha", label:"Map waypoint opacity (%)", values:&[40,60,80,100], default:100 },
];
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings(pub [i32; SPEC_COUNT]);
impl Default for Settings {
    fn default() -> Self { Self(std::array::from_fn(|i| SPECS[i].default)) }
}
impl Settings {
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let mut out = Self::default();
        for (i,spec) in SPECS.iter().enumerate() {
            if let Some(v) = get(spec.key) {
                // Preserve 0.1.0's string corner setting.
                let legacy = if i == MINI_CORNER {
                    ["top_left","top_right","bottom_left","bottom_right"].iter().position(|x| *x == v.trim()).map(|x| x as i32)
                } else { None };
                if let Some(n) = legacy.or_else(|| v.trim().parse().ok()).filter(|n| spec.values.contains(n)) { out.0[i] = n; }
            }
        }
        out
    }
    pub fn cycle(&mut self, index: usize) -> Option<(&'static str, String)> {
        let spec = SPECS.get(index)?;
        let old = spec.values.iter().position(|v| *v == self.0[index]).unwrap_or(0);
        self.0[index] = spec.values[(old + 1) % spec.values.len()];
        Some((spec.key, self.0[index].to_string()))
    }
    pub fn label(&self, i: usize) -> String {
        let spec = &SPECS[i];
        let value = if i == MINI_CORNER {
            ["Top left","Top right","Bottom left","Bottom right"][self.0[i] as usize].into()
        } else if spec.values == &[0,1] {
            if self.0[i] == 0 { "OFF".into() } else { "ON".into() }
        } else { self.0[i].to_string() };
        format!("{}: {}", spec.label, value)
    }
    pub fn on(&self, i: usize) -> bool { self.0[i] != 0 }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pixel { pub rgb: u32, pub height: i32 }
#[derive(Clone)]
pub struct Tile { pub pixels: Vec<Pixel>, pub dirty: bool, pub touched: u64 }
impl Tile {
    fn empty() -> Self { Self { pixels: vec![Pixel::default();TILE_CELLS], dirty:false, touched:0 } }
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = b"PMT2".to_vec();
        for p in &self.pixels { bytes.extend(p.rgb.to_le_bytes()); bytes.extend(p.height.to_le_bytes()); }
        bytes
    }
    pub fn decode(bytes: &[u8]) -> Result<Self,String> {
        if bytes.len() != 4 + TILE_CELLS * 8 || &bytes[..4] != b"PMT2" { return Err("invalid exploration tile (preserved, not overwritten)".into()); }
        let mut out = Self::empty();
        for (i,p) in out.pixels.iter_mut().enumerate() {
            let start = 4 + i*8;
            p.rgb = u32::from_le_bytes(bytes[start..start+4].try_into().map_err(|_| "bad color")?);
            p.height = i32::from_le_bytes(bytes[start+4..start+8].try_into().map_err(|_| "bad height")?);
        }
        Ok(out)
    }
}
pub fn tile_address(x: i32,z: i32) -> ((i32,i32),usize) {
    ((x.div_euclid(TILE_SIDE),z.div_euclid(TILE_SIDE)),
     (z.rem_euclid(TILE_SIDE)*TILE_SIDE+x.rem_euclid(TILE_SIDE)) as usize)
}
pub fn hash_id(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for b in value.bytes() { hash ^= u64::from(b); hash=hash.wrapping_mul(0x100000001b3); }
    format!("{hash:016x}")
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(),String> {
    let parent = path.parent().ok_or("no parent directory")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("pending");
    let mut file = File::create(&tmp).map_err(|e| e.to_string())?;
    file.write_all(bytes).and_then(|_|file.sync_all()).map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(&tmp,path).map_err(|e| e.to_string())
}
fn read_limited(path: &Path, limit: usize) -> Result<Option<Vec<u8>>,String> {
    let file = match File::open(path) {
        Ok(f)=>f,
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(None),
        Err(e)=>return Err(e.to_string()),
    };
    let mut bytes=Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    if bytes.len()>limit { return Err("map data exceeds size limit".into()); }
    Ok(Some(bytes))
}
pub const COLORS: [u32;8] = [0xffd45a,0xff6b6b,0x73d97e,0x64b5ff,0xc49bff,0xffa34d,0x67e0d2,0xffffff];
pub const ICONS: [&str;4] = ["Diamond","Pin","Cross","Square"];
#[derive(Clone,Debug,PartialEq)]
pub struct Waypoint {
    pub id:u64, pub name:String, pub pos:[f64;3], pub color:u32,
    pub icon:usize, pub enabled:bool, pub in_world:bool,
}
impl Waypoint {
    pub fn valid(&self) -> bool {
        !self.name.trim().is_empty() && self.name.chars().count()<=48
            && !self.name.chars().any(char::is_control)
            && self.pos.iter().all(|v|v.is_finite()&&v.abs()<=COORD_LIMIT)
            && self.icon<ICONS.len() && self.color<=0xffffff
    }
}
fn hex_text(text:&str)->String { text.as_bytes().iter().map(|b|format!("{b:02x}")).collect() }
fn unhex(text:&str)->Option<String> {
    if text.len()%2!=0 || text.len()>512 { return None; }
    let bytes=(0..text.len()).step_by(2).map(|i|u8::from_str_radix(text.get(i..i+2)?,16).ok()).collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}
pub fn encode_waypoints(points:&[Waypoint])->Vec<u8> {
    let mut text=String::from("PMW2\n");
    for p in points {
        text.push_str(&format!("{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",p.id,hex_text(&p.name),p.pos[0],p.pos[1],p.pos[2],p.color,p.icon,u8::from(p.enabled),u8::from(p.in_world)));
    }
    text.into_bytes()
}
pub fn decode_waypoints(bytes:&[u8])->Result<Vec<Waypoint>,String> {
    let text=std::str::from_utf8(bytes).map_err(|_|"waypoint data is not UTF-8")?;
    let mut lines=text.lines();
    if lines.next()!=Some("PMW2") { return Err("unknown waypoint format".into()); }
    let mut ids=BTreeSet::new();
    let mut out=Vec::new();
    for line in lines {
        let f:Vec<_>=line.split('\t').collect();
        if f.len()!=9 || out.len()>=MAX_WAYPOINTS { return Err("invalid waypoint record count".into()); }
        let parse=||->Option<Waypoint> {
            if !["0","1"].contains(&f[7])||!["0","1"].contains(&f[8]) { return None; }
            Some(Waypoint {id:f[0].parse().ok()?,name:unhex(f[1])?,pos:[f[2].parse().ok()?,f[3].parse().ok()?,f[4].parse().ok()?],color:f[5].parse().ok()?,icon:f[6].parse().ok()?,enabled:f[7]=="1",in_world:f[8]=="1"})
        };
        let p=parse().filter(Waypoint::valid).ok_or("invalid waypoint (file preserved)")?;
        if !ids.insert(p.id) { return Err("duplicate waypoint ID".into()); }
        out.push(p);
    }
    Ok(out)
}
pub struct Store {
    pub root:PathBuf, pub identity:String, pub tiles:BTreeMap<(i32,i32),Tile>,
    known:BTreeSet<(i32,i32)>,
    blocked:BTreeSet<(i32,i32)>, pub waypoints:Vec<Waypoint>, clock:u64,
    pub error:Option<String>, waypoint_readonly:bool,
}
impl Store {
    pub fn open(base:&Path,world:&str,dimension:&str)->Result<Self,String> {
        let identity=format!("{}:{}{}:{}",world.len(),world,dimension.len(),dimension);
        let root=base.join("atlas-v2").join(hash_id(&identity));
        let id_path=root.join("identity.txt");
        if let Some(existing)=read_limited(&id_path,16384)? {
            if existing!=identity.as_bytes() { return Err("map identity collision: refusing to mix worlds".into()); }
        } else { atomic_write(&id_path,identity.as_bytes())?; }
        let (waypoints,error,waypoint_readonly)=match read_limited(&root.join("waypoints.txt"),262144).and_then(|b|b.map_or(Ok(Vec::new()),|b|decode_waypoints(&b))) {
            Ok(p)=>(p,None,false),Err(e)=>(Vec::new(),Some(e),true),
        };
        let mut known=BTreeSet::new();
        for entry in fs::read_dir(&root).map_err(|e|e.to_string())? {
            let entry=entry.map_err(|e|e.to_string())?;
            if let Some(name)=entry.file_name().to_str().and_then(|s|s.strip_suffix(".pmt")) {
                if let Some((x,z))=name.split_once('_') {
                    if let (Ok(x),Ok(z))=(x.parse::<i32>(),z.parse::<i32>()) {known.insert((x,z));}
                }
            }
        }
        Ok(Self {root,identity,tiles:BTreeMap::new(),known,blocked:BTreeSet::new(),waypoints,clock:0,error,waypoint_readonly})
    }
    fn path(&self,key:(i32,i32))->PathBuf { self.root.join(format!("{}_{}.pmt",key.0,key.1)) }
    fn load(&mut self,key:(i32,i32))->bool {
        self.clock=self.clock.wrapping_add(1);
        if self.blocked.contains(&key) { return false; }
        if !self.tiles.contains_key(&key) {
            if self.tiles.len()>=CACHE_TILES {
                let oldest=self.tiles.iter().min_by_key(|(_,t)|t.touched).map(|(k,_)|*k);
                if let Some(k)=oldest {
                    if let Err(e)=self.save_tile(k) { self.error=Some(e); return false; }
                    self.tiles.remove(&k);
                }
            }
            let tile=match read_limited(&self.path(key),4+TILE_CELLS*8).and_then(|b|b.map_or(Ok(Tile::empty()),|b|Tile::decode(&b))) {
                Ok(t)=>t,Err(e)=>{self.error=Some(e);self.blocked.insert(key);return false;}
            };
            self.tiles.insert(key,tile);
        }
        if let Some(tile)=self.tiles.get_mut(&key) { tile.touched=self.clock; }
        true
    }
    pub fn get(&mut self,x:i32,z:i32)->Option<Pixel> {
        let (key,i)=tile_address(x,z);
        // Panning through blank space must not allocate tiles or perform file I/O.
        if !self.known.contains(&key)&&!self.tiles.contains_key(&key) {return None;}
        if !self.load(key) {return None;}
        self.tiles.get(&key).map(|t|t.pixels[i]).filter(|p|p.rgb!=0)
    }
    pub fn set(&mut self,x:i32,z:i32,pixel:Pixel) {
        let (key,i)=tile_address(x,z);
        if !self.load(key) {return;}
        self.known.insert(key);
        if let Some(tile)=self.tiles.get_mut(&key) {
            if tile.pixels[i]!=pixel { tile.pixels[i]=pixel;tile.dirty=true; }
        }
    }
    fn save_tile(&mut self,key:(i32,i32))->Result<(),String> {
        if self.tiles.get(&key).is_some_and(|t|t.dirty) {
            let bytes=self.tiles[&key].encode();
            atomic_write(&self.path(key),&bytes)?;
            if let Some(tile)=self.tiles.get_mut(&key) {tile.dirty=false;}
        }
        Ok(())
    }
    pub fn flush(&mut self)->Result<(),String> {
        let keys:Vec<_>=self.tiles.iter().filter(|(_,t)|t.dirty).map(|(k,_)|*k).collect();
        for key in keys {self.save_tile(key)?;}
        Ok(())
    }
    pub fn save_waypoint(&mut self,point:Waypoint)->Result<(),String> {
        if self.waypoint_readonly {return Err("waypoint file is unreadable; original preserved".into());}
        if !point.valid() {return Err("Name, coordinates or marker appearance is invalid".into());}
        let mut next=self.waypoints.clone();
        if let Some(p)=next.iter_mut().find(|p|p.id==point.id) {*p=point;}
        else if next.len()<MAX_WAYPOINTS {next.push(point);}
        else {return Err("Waypoint limit reached (256 per dimension)".into());}
        atomic_write(&self.root.join("waypoints.txt"),&encode_waypoints(&next))?;
        self.waypoints=next;
        Ok(())
    }
    pub fn remove_waypoint(&mut self,id:u64)->Result<(),String> {
        if self.waypoint_readonly {return Err("waypoint file is unreadable".into());}
        let next:Vec<_>=self.waypoints.iter().filter(|p|p.id!=id).cloned().collect();
        atomic_write(&self.root.join("waypoints.txt"),&encode_waypoints(&next))?;
        self.waypoints=next;
        Ok(())
    }
    pub fn next_id(&self)->u64 {self.waypoints.iter().map(|p|p.id).max().unwrap_or(0).saturating_add(1)}
}

#[derive(Clone,Copy,Debug,Default,PartialEq)]
pub struct Rect {pub x:f32,pub y:f32,pub w:f32,pub h:f32}
impl Rect {
    pub fn contains(self,p:[f32;2])->bool {p[0]>=self.x&&p[1]>=self.y&&p[0]<self.x+self.w&&p[1]<self.y+self.h}
}
#[derive(Clone,Copy,Debug,PartialEq)]
pub struct View {pub center:[f64;2],pub bpp:f64}
impl Default for View {fn default()->Self {Self {center:[0.0;2],bpp:2.0}}}
impl View {
    pub fn world(self,rect:Rect,p:[f32;2])->[f64;2] {
        [self.center[0]+f64::from(p[0]-rect.x-rect.w/2.0)*self.bpp,
         self.center[1]+f64::from(p[1]-rect.y-rect.h/2.0)*self.bpp]
    }
    pub fn screen(self,rect:Rect,p:[f64;2])->[f32;2] {
        [rect.x+rect.w/2.0+((p[0]-self.center[0])/self.bpp) as f32,
         rect.y+rect.h/2.0+((p[1]-self.center[1])/self.bpp) as f32]
    }
    pub fn zoom(&mut self,rect:Rect,mouse:[f32;2],steps:f64) {
        if !steps.is_finite() {return;}
        let anchor=self.world(rect,mouse);
        self.bpp=(self.bpp*2.0f64.powf(-steps.clamp(-4.0,4.0)*0.5)).clamp(0.25,32.0);
        let after=self.world(rect,mouse);
        self.center[0]=(self.center[0]+anchor[0]-after[0]).clamp(-COORD_LIMIT,COORD_LIMIT);
        self.center[1]=(self.center[1]+anchor[1]-after[1]).clamp(-COORD_LIMIT,COORD_LIMIT);
    }
    pub fn pan(&mut self,delta:[f32;2]) {
        for (i,d) in delta.into_iter().enumerate() {
            if d.is_finite() {self.center[i]=(self.center[i]-f64::from(d)*self.bpp).clamp(-COORD_LIMIT,COORD_LIMIT);}
        }
    }
}
pub fn default_hotkey(m_used:bool)->&'static str {
    if m_used {"key.keyboard.j"} else {"key.keyboard.m"}
}
pub fn choose_hotkey(explicit:Option<&str>,bindings:&[String])->(String,bool) {
    let fallback=default_hotkey(bindings.iter().any(|v|v=="key.keyboard.m"));
    let selected=explicit.unwrap_or(fallback).to_owned();
    let conflict=bindings.contains(&selected);
    (selected,conflict)
}
pub fn reveal_offsets(radius:i32)->Vec<[i32;2]> {
    let radius=radius.clamp(1,64);
    let mut offsets=Vec::new();
    for z in -radius..=radius {for x in -radius..=radius {
        if x*x+z*z<=radius*radius {offsets.push([x,z]);}
    }}
    offsets.sort_by_key(|p|(p[0]*p[0]+p[1]*p[1],p[1],p[0]));
    offsets
}
pub fn in_reveal_radius(point:[i32;2],player:[f64;3],radius:i32)->bool {
    let dx=f64::from(point[0])+0.5-player[0];let dz=f64::from(point[1])+0.5-player[2];
    dx*dx+dz*dz<=f64::from(radius*radius)
}
pub fn distance(a:[f64;3],b:[f64;3])->f64 {
    a.iter().zip(b).map(|(a,b)|(a-b).powi(2)).sum::<f64>().sqrt()
}
#[derive(Clone,Debug)]
pub struct Editor {
    pub id:u64,pub fields:[String;4],pub focus:usize,pub color:usize,pub icon:usize,
    pub enabled:bool,pub in_world:bool,pub confirm_delete:bool,
}
impl Editor {
    pub fn from_point(p:&Waypoint)->Self {
        Self {id:p.id,fields:[p.name.clone(),p.pos[0].to_string(),p.pos[1].to_string(),p.pos[2].to_string()],
            focus:0,color:COLORS.iter().position(|v|*v==p.color).unwrap_or(0),icon:p.icon,
            enabled:p.enabled,in_world:p.in_world,confirm_delete:false}
    }
    pub fn point(&self)->Result<Waypoint,String> {
        let coord=|i:usize|self.fields[i].trim().parse::<f64>().map_err(|_|"Coordinates must be numbers".to_owned());
        let p=Waypoint {id:self.id,name:self.fields[0].trim().to_owned(),pos:[coord(1)?,coord(2)?,coord(3)?],
            color:COLORS[self.color],icon:self.icon,enabled:self.enabled,in_world:self.in_world};
        if !p.valid() {return Err("Use a name (1-48 characters) and finite coordinates inside world bounds".into());}
        Ok(p)
    }
    pub fn character(&mut self,c:char) {
        if c.is_control() {return;}
        let max=if self.focus==0 {48}else{24};
        let field=&mut self.fields[self.focus];
        if field.chars().count()<max && (self.focus==0||c.is_ascii_digit()||c=='.'||c=='-') {field.push(c);}
    }
}
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub enum Page {Map,Waypoints,Settings,Editor}
#[derive(Clone,Debug)]
pub enum Action {Close,Zoom(f64),Center,New([f64;2]),Page(Page),Edit(u64),Save,Delete,Setting(usize)}
pub struct State {
    pub config:Settings,pub config_loaded:bool,pub store:Option<Store>,pub pending_stores:Vec<Store>,
    pub player:[f64;3],pub yaw:f32,pub dimension:String,pub world:String,
    pub hotkey:String,pub hotkey_warning:bool,pub tick:u64,pub view:View,pub page:Page,
    pub page_offset:usize,pub editor:Option<Editor>,pub actions:Vec<Action>,pub status:String,
    pub open:bool,pub dragging:bool,pub rect:Rect,pub gui:[f32;2],pub image_dirty:bool,
    pub image_ready:bool,pub uploaded_view:View,pub uploaded_mini:View,pub uploaded_rect:Rect,
    pub texture_size:[u32;2],pub mini_texture_side:u32,pub offsets:Vec<[i32;2]>,pub scan_index:usize,pub scan_origin:[i32;2],
    pub scan_radius:i32,pub last_center:[i32;2],
}
impl Default for State {
    fn default()->Self {
        Self {config:Settings::default(),config_loaded:false,store:None,pending_stores:Vec::new(),
            player:[0.0;3],yaw:0.0,dimension:String::new(),world:String::new(),
            hotkey:"key.keyboard.m".into(),hotkey_warning:false,tick:0,view:View::default(),page:Page::Map,
            page_offset:0,editor:None,actions:Vec::new(),status:String::new(),open:false,dragging:false,
            rect:Rect::default(),gui:[0.0;2],image_dirty:true,image_ready:false,
            uploaded_view:View::default(),uploaded_mini:View::default(),uploaded_rect:Rect::default(),
            texture_size:[512,512],mini_texture_side:128,offsets:reveal_offsets(32),scan_index:0,scan_origin:[0;2],
            scan_radius:32,last_center:[i32::MIN;2]}
    }
}
impl State {
    pub fn notice(&mut self,result:Result<(),String>,success:&str) {
        self.status=match result {Ok(())=>success.into(),Err(e)=>format!("Save failed: {e}")};
    }
    pub fn begin_waypoint(&mut self,at:[f64;2]) {
        if let Some(store)=self.store.as_mut() {
            let y=store.get(at[0].floor() as i32,at[1].floor() as i32).map_or(self.player[1],|p|f64::from(p.height)+1.0);
            let p=Waypoint {id:store.next_id(),name:"Waypoint".into(),pos:[at[0],y,at[1]],color:COLORS[0],icon:0,enabled:true,in_world:false};
            self.editor=Some(Editor::from_point(&p));self.page=Page::Editor;
        }
    }
    pub fn flush(&mut self) {
        if let Some(store)=self.store.as_mut() {
            if let Err(e)=store.flush() {self.status=format!("Exploration save failed: {e}");}
        }
        // Preserve dirty old worlds in memory if a world-change save failed.
        self.pending_stores.retain_mut(|old|old.flush().is_err());
    }
    pub fn switch_world(&mut self,base:&Path,world:&str,dimension:&str)->bool {
        if self.world==world&&self.dimension==dimension&&self.store.is_some() {return true;}
        self.flush();
        if let Some(mut old)=self.store.take() {
            if old.flush().is_err() {self.pending_stores.push(old);}
        }
        self.world=world.into();self.dimension=dimension.into();
        self.page=Page::Map;self.editor=None;self.dragging=false;self.image_ready=false;
        self.actions.clear();self.view.center=[self.player[0],self.player[2]];
        self.image_dirty=true;self.scan_index=0;self.last_center=[i32::MIN;2];
        // Don't reopen stale on-disk state over an unsaved store for this world.
        let identity=format!("{}:{}{}:{}",world.len(),world,dimension.len(),dimension);
        if let Some(i)=self.pending_stores.iter().position(|s|s.identity==identity) {
            self.store=Some(self.pending_stores.remove(i));return true;
        }
        match Store::open(base,world,dimension) {
            Ok(store)=>{self.store=Some(store);true}
            Err(e)=>{self.status=format!("Map data unavailable: {e}");false}
        }
    }
}
thread_local! {pub static STATE:RefCell<State>=RefCell::new(State::default());}
pub fn with_state<R>(f:impl FnOnce(&mut State)->R)->R {STATE.with(|s|f(&mut s.borrow_mut()))}
