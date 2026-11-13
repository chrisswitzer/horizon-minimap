// Horizon-specific integration. All source-sensitive access is isolated here.
use crate::game::{Game, paper_minimap as m};
use horizon_client::hud::{HudAtlas,HudBatch};
use horizon_client::extras_hud::Corners;
use horizon_client::extras::Corner;

// Registered vanilla keys, including movement, GUI and miscellaneous mappings.
const VANILLA_KEYS:&[&str]=&[
 "key.attack","key.use","key.forward","key.back","key.left","key.right","key.jump","key.sneak",
 "key.sprint","key.drop","key.inventory","key.chat","key.playerlist","key.pickItem","key.command",
 "key.socialInteractions","key.screenshot","key.togglePerspective","key.smoothCamera","key.fullscreen",
 "key.spectatorOutlines","key.swapOffhand","key.advancements","key.saveToolbarActivator",
 "key.loadToolbarActivator","key.hotbar.1","key.hotbar.2","key.hotbar.3","key.hotbar.4",
 "key.hotbar.5","key.hotbar.6","key.hotbar.7","key.hotbar.8","key.hotbar.9"
];
pub fn hotkey(game:&Game<'_>)->(String,bool) {
    let mut bindings:Vec<String>=VANILLA_KEYS.iter().map(|name|game.settings.keys.get(name).to_owned()).collect();
    // Include configured bindings for every other native mod, even disabled ones.
    for (id,keys) in &game.settings.mods.keys {
        if id!=m::ID {bindings.extend(keys.values().cloned());}
    }
    let mut mods=Vec::new();
    Game::native_mods(&mut mods);
    for entry in mods {
        if entry.id==m::ID {continue;}
        for key in entry.keys {
            bindings.push(game.native_mod_key(&entry.id,&key[0],&key[2]));
        }
    }
    let explicit=game.settings.mods.keys.get(m::ID).and_then(|keys|keys.get("full_map")).map(String::as_str);
    m::choose_hotkey(explicit,&bindings)
}
fn sample(game:&Game<'_>,x:i32,z:i32,player_y:f64,dimension:&str)->Option<m::Pixel> {
    use horizon_client::blocks::{Fluid,MAP_COLORS};
    // Never create/generate/request chunks to reveal the map.
    if !game.world.chunks.contains_key(&(x.div_euclid(16),z.div_euclid(16))) {return None;}
    let bottom=game.world.bottom_y();
    let top=game.world.world_surface(x,z,|s|game.res.facts().is_air(s));
    // Under the Nether roof, map the floor near the player's vertical level.
    let ceiling=dimension.contains("nether")&&player_y<127.0;
    let mut y=if ceiling {(player_y.floor() as i32+1).min(top-1)}else{top-1};
    while y>bottom && game.res.facts().get(game.world.block(x,y,z)).map_color==0 {y-=1;}
    let fact=game.res.facts().get(game.world.block(x,y,z));
    let mut color=usize::from(fact.map_color);
    let mut shade=220u32;
    if fact.fluid!=Fluid::None {
        color=if fact.fluid==Fluid::Lava {4}else{12};
        let mut depth=0;
        while depth<16 && y-depth>bottom && game.res.facts().get(game.world.block(x,y-depth,z)).fluid!=Fluid::None {depth+=1;}
        if color==12 {shade=if depth>5 {135}else if depth>2 {180}else{220};}
    } else {
        let north=game.world.world_surface(x,z-1,|s|game.res.facts().is_air(s))-1;
        if game.world.chunks.contains_key(&(x.div_euclid(16),(z-1).div_euclid(16))) && !ceiling {
            shade=if y>north {255}else if y<north {180}else{220};
        }
    }
    // Known empty/void columns are paper, distinct from unvisited pixels.
    let rgb=MAP_COLORS.get(color).copied().unwrap_or(0);
    let rgb=if color==0 {0xd8c89f}else{
        let channel=|shift:u32|((rgb>>shift&255)*shade/255)<<shift;
        channel(16)|channel(8)|channel(0)
    };
    Some(m::Pixel {rgb:0xff000000|rgb,height:y})
}
pub fn tick(game:&mut Game<'_>) {
    if !game.native_window() {return;}
    let on=game.native_on(m::ID);
    if !on {
        if game.native_mod_screen()==Some(m::ID) {game.native_mod_screen_close(m::ID);}
        m::with_state(|s|{if s.store.is_some(){s.flush();}s.open=false;s.image_ready=false;});
        return;
    }
    let Some(player)=game.native_player() else {
        m::with_state(|s|{
            s.flush();
            if let Some(mut old)=s.store.take() {if old.flush().is_err(){s.pending_stores.push(old);}}
            s.world.clear();s.dimension.clear();s.open=false;s.image_ready=false;s.actions.clear();s.editor=None;
        });
        return;
    };
    let Some(client)=game.native_client_state() else {return;};
    let Some(world)=client.world else {return;};
    // Debug serialization includes the variant (Singleplayer/Server) and the
    // complete host/save identity; dimension is separately included by Store.
    let world_id=format!("{world:?}");
    let dimension=player.dimension.to_string();
    let base=game.native_data_dir(m::ID);
    let (chosen,conflict)=hotkey(game);
    let opened=game.native_mod_screen()==Some(m::ID);
    let mut close=false;
    m::with_state(|s|{
        s.tick=s.tick.wrapping_add(1);
        s.hotkey=chosen;s.hotkey_warning=conflict;
        if !s.config_loaded {s.config=m::Settings::read(|key|game.native_setting(m::ID,key));s.config_loaded=true;}
        s.player=[player.x,player.y,player.z];s.yaw=player.yaw;
        if !s.switch_world(&base,&world_id,&dimension) {return;}
        if s.open&&!opened {s.flush();s.dragging=false;s.editor=None;}
        s.open=opened;s.gui=client.gui_size;
        if !opened {s.actions.clear();}
        for action in std::mem::take(&mut s.actions) {
            match action {
                m::Action::Close=>{close=true;s.flush();}
                m::Action::Zoom(n)=>{s.view.zoom(s.rect,[s.rect.x+s.rect.w/2.0,s.rect.y+s.rect.h/2.0],n);s.image_dirty=true;}
                m::Action::Center=>{s.view.center=[player.x,player.z];s.image_dirty=true;}
                m::Action::New(at)=>s.begin_waypoint(at),
                m::Action::Page(page)=>{s.page=page;s.page_offset=0;s.editor=None;}
                m::Action::Edit(id)=>{
                    if let Some(p)=s.store.as_ref().and_then(|store|store.waypoints.iter().find(|p|p.id==id)) {
                        s.editor=Some(m::Editor::from_point(p));s.page=m::Page::Editor;
                    }
                }
                m::Action::Save=>{
                    let result=s.editor.as_ref().ok_or("No waypoint selected".into()).and_then(m::Editor::point)
                        .and_then(|p|s.store.as_mut().ok_or("No world loaded".into())?.save_waypoint(p));
                    if result.is_ok() {s.page=m::Page::Waypoints;s.editor=None;}
                    s.notice(result,"Waypoint saved");
                }
                m::Action::Delete=>{
                    let result=match (&s.editor,&mut s.store) {
                        (Some(e),Some(store)) if e.confirm_delete=>store.remove_waypoint(e.id),
                        _=>Err("Confirm waypoint deletion first".into()),
                    };
                    if result.is_ok(){s.page=m::Page::Waypoints;s.editor=None;}
                    s.notice(result,"Waypoint removed");
                }
                m::Action::Setting(i)=>{
                    let old=s.config;
                    if let Some((key,value))=s.config.cycle(i) {
                        let result=m::atomic_write(&game.native_data_dir(m::ID).join(key),value.as_bytes());
                        if result.is_err(){s.config=old;}
                        s.notice(result,"Setting saved");
                        s.image_dirty=true;
                    }
                }
            }
        }
        let radius=s.config.0[m::REVEAL_RADIUS];
        if radius!=s.scan_radius {s.offsets=m::reveal_offsets(radius);s.scan_radius=radius;s.scan_index=0;}
        let center=[player.x.floor() as i32,player.z.floor() as i32];
        let moved=(center[0]-s.scan_origin[0]).abs()+(center[1]-s.scan_origin[1]).abs();
        if s.scan_index>=s.offsets.len()||moved>radius/2||s.last_center==[i32::MIN;2] {
            s.scan_index=0;s.scan_origin=center;
        }
        let budget=s.config.0[m::SAMPLE_BUDGET] as usize;
        if let Some(store)=s.store.as_mut() {
            for _ in 0..budget {
                let Some(offset)=s.offsets.get(s.scan_index) else{break;};
                s.scan_index+=1;
                let at=[s.scan_origin[0]+offset[0],s.scan_origin[1]+offset[1]];
                if !m::in_reveal_radius(at,s.player,radius) {continue;}
                if let Some(pixel)=sample(game,at[0],at[1],player.y,&dimension) {
                    // Old/unloaded terrain remains unchanged; only actual samples reveal.
                    store.set(at[0],at[1],pixel);s.image_dirty=true;
                }
            }
            if let Some(error)=store.error.take() {s.status=format!("Map data: {error}");}
        }
        if center!=s.last_center {s.image_dirty=true;s.last_center=center;}
        if s.tick%(20*s.config.0[m::SAVE_SECONDS] as u64)==0 {s.flush();}
    });
    if close {game.native_mod_screen_close(m::ID);}
    upload(game);
}
pub fn key(game:&mut Game<'_>,key:&str,pressed:bool) {
    if !pressed||!game.native_on(m::ID)||game.native_player().is_none() {return;}
    let Some(player)=game.native_player()else{return;};
    let (chosen,conflict)=hotkey(game);
    if key!=chosen {return;}
    if game.native_mod_screen()==Some(m::ID) {game.native_mod_screen_close(m::ID);return;}
    if !game.native_playing() {return;}
    if conflict {
        game.native_chat("Paper Map: both the automatic/default key and another control conflict. Rebind Full map in Options > Mods > Paper Minimap.");
        return;
    }
    if game.native_mod_screen_open(m::ID) {
        m::with_state(|s|{
            s.page=m::Page::Map;s.editor=None;s.actions.clear();s.dragging=false;
            if s.config.on(m::FOLLOW)||!s.image_ready {s.view.center=[player.x,player.z];}
            s.open=true;s.image_dirty=true;
        });
    }
}
fn upload(game:&mut Game<'_>) {
    let request=m::with_state(|s|{
        if !s.image_dirty||s.store.is_none() {return None;}
        // Cap expensive raster rebuilds at 10 Hz while walking/exploring.
        if s.tick%2!=0 && s.image_ready {return None;}
        let layout=Layout::new(s.gui,s.config);
        let full_rect=layout.map;
        s.rect=full_rect;
        let width=full_rect.w.ceil().clamp(1.0,512.0) as u32;
        let height=full_rect.h.ceil().clamp(1.0,512.0) as u32;
        let mini_side=s.config.0[m::MINI_SIZE].min(256) as u32;
        let miniature=m::View {center:[s.player[0].floor()+0.5,s.player[2].floor()+0.5],bpp:f64::from(s.config.0[m::MINI_ZOOM])};
        let mini_rect=m::Rect {x:0.0,y:0.0,w:mini_side as f32,h:mini_side as f32};
        let mut rgba=vec![0u8;768*512*4];
        let store=s.store.as_mut()?;
        let paint=|store:&mut m::Store,pixels:&mut [u8],view:m::View,rect:m::Rect,w:u32,h:u32,offset:u32,alpha:u8|{
            for row in 0..h {for col in 0..w {
                let screen=[rect.x+(col as f32+0.5)*rect.w/w as f32,rect.y+(row as f32+0.5)*rect.h/h as f32];
                let at=view.world(rect,screen);
                let pixel=store.get(at[0].floor() as i32,at[1].floor() as i32);
                let color=pixel.map_or(if (col/8+row/8)%2==0 {0xd5c49b}else{0xd1bf95},|p|p.rgb);
                let i=((row*768+col+offset)*4) as usize;
                pixels[i..i+4].copy_from_slice(&[(color>>16) as u8,(color>>8) as u8,color as u8,alpha]);
            }}
        };
        if s.open {paint(store,&mut rgba,s.view,full_rect,width,height,0,255);}
        paint(store,&mut rgba,miniature,mini_rect,mini_side,mini_side,512,(s.config.0[m::MAP_ALPHA]*255/100) as u8);
        s.uploaded_view=s.view;s.uploaded_rect=full_rect;s.uploaded_mini=miniature;
        s.texture_size=[width,height];
        s.mini_texture_side=mini_side;
        Some(rgba)
    });
    if let Some(pixels)=request {
        let ok=game.native_mod_image(768,512,&pixels);
        m::with_state(|s|{s.image_ready=ok;s.image_dirty=!ok;if !ok{s.status="Map image upload unavailable".into();}});
    }
}
pub struct Layout {pub scale:f32,pub map:m::Rect,pub toolbar:f32,pub panel:m::Rect}
impl Layout {
    pub fn new(gui:[f32;2],config:m::Settings)->Self {
        let scale=(config.0[m::GUI_SCALE] as f32/100.0).min((gui[0]/560.0).max(0.35)).min((gui[1]/360.0).max(0.35));
        let edge=10.0*scale;
        let map=m::Rect{x:edge,y:38.0*scale,w:(gui[0]-2.0*edge).max(1.0),h:(gui[1]-64.0*scale).max(1.0)};
        let panel=m::Rect{x:(gui[0]-440.0*scale)/2.0,y:48.0*scale,w:440.0*scale,h:(gui[1]-80.0*scale).max(1.0)};
        Self{scale,map,toolbar:12.0*scale,panel}
    }
    pub fn toolbar_button(&self,i:usize)->m::Rect {
        m::Rect{x:(12.0+i as f32*77.0)*self.scale,y:self.toolbar,w:72.0*self.scale,h:18.0*self.scale}
    }
    pub fn row(&self,i:usize)->m::Rect {
        m::Rect{x:self.panel.x+8.0*self.scale,y:self.panel.y+(10.0+i as f32*23.0)*self.scale,w:self.panel.w-16.0*self.scale,h:20.0*self.scale}
    }
    pub fn rows(&self)->usize {((self.panel.h/self.scale-45.0)/23.0).floor().max(1.0) as usize}
}
fn fill(game:&Game<'_>,batch:&mut HudBatch,r:m::Rect,c:u32) {game.native_fill(batch,[r.x,r.y,r.w,r.h],c);}
fn text(game:&Game<'_>,batch:&mut HudBatch,t:&str,x:f32,y:f32,color:u32,scale:f32) {
    let start=batch.vertices.len();
    game.native_text(batch,t,[x,y],color,false);
    for v in &mut batch.vertices[start..] {v.pos[0]=x+(v.pos[0]-x)*scale;v.pos[1]=y+(v.pos[1]-y)*scale;}
}
fn button(game:&Game<'_>,batch:&mut HudBatch,r:m::Rect,label:&str,scale:f32,hover:bool) {
    fill(game,batch,r,if hover {0xffb59b6a}else{0xff917548});
    fill(game,batch,m::Rect{x:r.x+1.0,y:r.y+1.0,w:r.w-2.0,h:r.h-2.0},if hover{0xffe8d8ae}else{0xffd3c095});
    text(game,batch,label,r.x+4.0*scale,r.y+6.0*scale,0x352916,scale);
}
fn paper(game:&Game<'_>,batch:&mut HudBatch,r:m::Rect,border:f32,alpha:u32) {
    // Disjoint one-pixel rings: opacity must not compound across stacked fills.
    for i in 0..border.ceil() as i32 {
        let reach=border-i as f32;
        let color=if i==0{0x5c4125}else if i<3{0x9f7d4d}else{0xdcca9e};
        let c=(alpha<<24)|color;
        fill(game,batch,m::Rect{x:r.x-reach,y:r.y-reach,w:r.w+2.0*reach,h:1.0},c);
        fill(game,batch,m::Rect{x:r.x-reach,y:r.y+r.h+reach-1.0,w:r.w+2.0*reach,h:1.0},c);
        fill(game,batch,m::Rect{x:r.x-reach,y:r.y-reach+1.0,w:1.0,h:r.h+2.0*reach-2.0},c);
        fill(game,batch,m::Rect{x:r.x+r.w+reach-1.0,y:r.y-reach+1.0,w:1.0,h:r.h+2.0*reach-2.0},c);
    }
    if alpha==255{fill(game,batch,r,0xffd5c49b);}
}
fn icon(game:&Game<'_>,batch:&mut HudBatch,p:[f32;2],size:f32,color:u32,shape:usize) {
    let h=size/2.0;
    match shape {
        0=>{for row in 0..size.ceil() as i32 {let d=(row as f32-h).abs();fill(game,batch,m::Rect{x:p[0]-h+d,y:p[1]-h+row as f32,w:(size-2.0*d).max(1.0),h:1.0},color);}}
        1=>{fill(game,batch,m::Rect{x:p[0]-h,y:p[1]-h,w:size,h:size*0.65},color);fill(game,batch,m::Rect{x:p[0]-1.0,y:p[1],w:2.0,h:h+2.0},color);}
        2=>{fill(game,batch,m::Rect{x:p[0]-h,y:p[1]-1.0,w:size,h:2.0},color);fill(game,batch,m::Rect{x:p[0]-1.0,y:p[1]-h,w:2.0,h:size},color);}
        _=>fill(game,batch,m::Rect{x:p[0]-h,y:p[1]-h,w:size,h:size},color),
    }
}
fn markers(game:&Game<'_>,batch:&mut HudBatch,s:&m::State,view:m::View,rect:m::Rect,scale:f32) {
    let Some(store)=s.store.as_ref() else{return;};
    let size=s.config.0[m::MARKER_SIZE] as f32*scale;
    let alpha=(s.config.0[m::MARKER_ALPHA]*255/100) as u32;
    let mut label_rows:Vec<m::Rect>=Vec::new();
    for waypoint in &store.waypoints {
        if !waypoint.enabled {continue;}
        let p=view.screen(rect,[waypoint.pos[0],waypoint.pos[2]]);
        let safe=m::Rect{x:rect.x+size,y:rect.y+size,w:rect.w-2.0*size,h:rect.h-2.0*size};
        if !safe.contains(p) {continue;}
        icon(game,batch,p,size+2.0*scale,(alpha<<24)|0x342815,waypoint.icon);
        icon(game,batch,p,size,(alpha<<24)|waypoint.color,waypoint.icon);
        let mut label=if s.config.on(m::LABELS){waypoint.name.clone()}else{String::new()};
        if s.config.on(m::DISTANCES) {label.push_str(&format!(" {:.0}m",m::distance(s.player,waypoint.pos)));}
        if label.is_empty() {continue;}
        let w=game.native_text_width(&label)*scale+4.0*scale;
        let box_rect=m::Rect{x:(p[0]+size).min(rect.x+rect.w-w),y:p[1]-5.0*scale,w,h:12.0*scale};
        if box_rect.x<rect.x||box_rect.y<rect.y||box_rect.y+box_rect.h>rect.y+rect.h {continue;}
        if label_rows.iter().any(|r|r.x<box_rect.x+box_rect.w&&r.x+r.w>box_rect.x&&r.y<box_rect.y+box_rect.h&&r.y+r.h>box_rect.y) {continue;}
        label_rows.push(box_rect);
        if s.config.on(m::LABEL_BG){fill(game,batch,box_rect,0xcc2b2418);}
        text(game,batch,&label,box_rect.x+2.0*scale,box_rect.y+2.0*scale,waypoint.color,scale);
    }
    let p=view.screen(rect,[s.player[0],s.player[2]]);
    if rect.contains(p) {
        // Draw the loaded vanilla map arrow with Horizon's map UV orientation.
        let atlas:&HudAtlas=&game.gui.atlas;
        if let Some(r)=atlas.sprites.get("map/decorations/player").copied() {
            let angle=(s.yaw+180.0).to_radians();
            let (sin,cos)=angle.sin_cos();let h=4.0*scale;
            let point=|dx:f32,dy:f32|[p[0]+dx*cos-dy*sin,p[1]+dx*sin+dy*cos];
            batch.quad([point(-h,h),point(h,h),point(h,-h),point(-h,-h)],
                [[r.u0,r.v0],[r.u1,r.v0],[r.u1,r.v1],[r.u0,r.v1]],u32::MAX);
        } else {icon(game,batch,p,6.0*scale,0xffffffff,2);}
    }
}
pub fn hud(game:&mut Game<'_>,batch:&mut HudBatch,corners:&mut Corners) {
    if !game.native_on(m::ID)||game.native_player().is_none()||game.native_mod_screen().is_some() {return;}
    let Some(player)=game.native_player()else{return;};
    let world=game.native_client_state().and_then(|c|c.world).map(|w|format!("{w:?}"));
    m::with_state(|s|{
        if !s.config.on(m::MINI_SHOW)||!s.image_ready {return;}
        if s.dimension!=player.dimension.to_string()||world.as_deref()!=Some(s.world.as_str()){return;}
        let corner=[Corner::TopLeft,Corner::TopRight,Corner::BottomLeft,Corner::BottomRight][s.config.0[m::MINI_CORNER] as usize];
        if !game.native_corner_shown(corner) {return;}
        let border=s.config.0[m::BORDER] as f32;
        let size=(s.config.0[m::MINI_SIZE] as f32).min((batch.width-2.0*border-4.0).max(16.0)).min((batch.height-2.0*border-4.0).max(16.0));
        let (x,y)=corners.place(corner,size+border*2.0,size+border*2.0);
        let rect=m::Rect{x:x+border,y:y+border,w:size,h:size};
        let alpha=(s.config.0[m::MAP_ALPHA]*255/100) as u32;
        paper(game,batch,rect,border,alpha);
        let texture_side=s.mini_texture_side as f32;
        game.native_sprite(batch,horizon_client::hud::MOD_IMAGE_SPRITE,[512.0,0.0,texture_side,texture_side],[rect.x,rect.y,rect.w,rect.h]);
        let view=m::View{center:s.uploaded_mini.center,bpp:s.uploaded_mini.bpp*f64::from(texture_side/size)};
        markers(game,batch,s,view,rect,1.0);
        if s.config.on(m::NORTH) {text(game,batch,"N",rect.x+rect.w/2.0-3.0,rect.y+3.0,0xffffff,1.0);}
    });
}
pub fn screen(game:&Game<'_>,batch:&mut HudBatch) {
    if !game.native_on(m::ID)||game.native_mod_screen()!=Some(m::ID) {return;}
    m::with_state(|s|{
        let gui=[batch.width,batch.height];
        if s.gui!=gui {s.gui=gui;s.image_dirty=true;}
        let l=Layout::new(gui,s.config);s.rect=l.map;
        fill(game,batch,m::Rect{x:0.0,y:0.0,w:gui[0],h:gui[1]},0xff30281b);
        for (i,label) in ["Zoom +","Zoom -","Center","Add point","Waypoints","Settings","Close"].iter().enumerate() {
            let r=l.toolbar_button(i);button(game,batch,r,label,l.scale,r.contains(game.native_cursor()));
        }
        paper(game,batch,l.map,s.config.0[m::BORDER] as f32,255);
        if s.image_ready && s.uploaded_rect.w==l.map.w && s.uploaded_rect.h==l.map.h {
            game.native_sprite(batch,horizon_client::hud::MOD_IMAGE_SPRITE,[0.0,0.0,s.texture_size[0] as f32,s.texture_size[1] as f32],[l.map.x,l.map.y,l.map.w,l.map.h]);
            let view=m::View{center:s.uploaded_view.center,bpp:s.uploaded_view.bpp*f64::from(s.uploaded_rect.w/l.map.w)};
            markers(game,batch,s,view,l.map,l.scale);
        }
        if s.config.on(m::NORTH){text(game,batch,"N",l.map.x+l.map.w/2.0-3.0*l.scale,l.map.y+4.0*l.scale,0x342815,l.scale);}
        if s.page!=m::Page::Map {draw_panel(game,batch,s,&l);}
        let help=if s.status.is_empty() {
            format!("Scroll: zoom | Drag: pan | Right click: waypoint | 1 px = {:.2} blocks | {}",s.view.bpp,s.dimension)
        }else{s.status.clone()};
        let max=((gui[0]/(6.0*l.scale)) as usize).saturating_sub(2);
        let clipped:String=help.chars().take(max).collect();
        text(game,batch,&clipped,10.0*l.scale,gui[1]-14.0*l.scale,0xf0dfb2,l.scale);
    });
}
fn draw_panel(game:&Game<'_>,batch:&mut HudBatch,s:&m::State,l:&Layout) {
    fill(game,batch,l.panel,0xffe2d1aa);
    match s.page {
        m::Page::Settings=>{
            let count=l.rows();
            for (row,i) in (s.page_offset..m::SPEC_COUNT).take(count).enumerate() {
                button(game,batch,l.row(row),&s.config.label(i),l.scale,l.row(row).contains(game.native_cursor()));
            }
            text(game,batch,"Scroll to see all settings. Click to change.",l.panel.x+8.0*l.scale,l.panel.y+l.panel.h-17.0*l.scale,0x352916,l.scale);
        }
        m::Page::Waypoints=>{
            if let Some(store)=&s.store {
                for (row,p) in store.waypoints.iter().skip(s.page_offset).take(l.rows()).enumerate() {
                    let label=format!("{} ({:.0}, {:.0}, {:.0}) {}",p.name,p.pos[0],p.pos[1],p.pos[2],if p.enabled{""}else{"[hidden]"});
                    button(game,batch,l.row(row),&label.chars().take(58).collect::<String>(),l.scale,l.row(row).contains(game.native_cursor()));
                }
                if store.waypoints.is_empty(){text(game,batch,"No waypoints. Click Add point to create one.",l.panel.x+8.0*l.scale,l.panel.y+12.0*l.scale,0x352916,l.scale);}
            }
            text(game,batch,"Click a waypoint to edit. Scroll for more.",l.panel.x+8.0*l.scale,l.panel.y+l.panel.h-17.0*l.scale,0x352916,l.scale);
        }
        m::Page::Editor=>{
            if let Some(e)=&s.editor {
                for i in 0..4 {
                    let value=format!("{}: {}{}",["Name","X","Y","Z"][i],e.fields[i],if e.focus==i{"_"}else{""});
                    button(game,batch,l.row(i),&value,l.scale,e.focus==i);
                }
                for (row,label) in [
                    format!("Color: #{:06X}",m::COLORS[e.color]),
                    format!("Icon: {}",m::ICONS[e.icon]),
                    format!("Show on map: {}",if e.enabled{"ON"}else{"OFF"}),
                    format!("Allow in-world marker: {} (global switch also required)",if e.in_world{"ON"}else{"OFF"}),
                    "Save waypoint".into(),
                    if e.confirm_delete{"Confirm DELETE waypoint".into()}else{"Delete waypoint...".into()},
                ].iter().enumerate(){button(game,batch,l.row(row+4),label,l.scale,false);}
            }
        }
        _=>{}
    }
}
pub fn click(game:&Game<'_>,button_index:i32) {
    if game.native_mod_screen()!=Some(m::ID)||!game.native_on(m::ID){return;}
    let mouse=game.native_cursor();
    m::with_state(|s|{
        let l=Layout::new(s.gui,s.config);
        if button_index==0 {
            for i in 0..7 {
                if l.toolbar_button(i).contains(mouse) {
                    let action=match i {
                        0=>m::Action::Zoom(1.0),1=>m::Action::Zoom(-1.0),2=>m::Action::Center,
                        3=>m::Action::New([s.player[0],s.player[2]]),
                        4=>m::Action::Page(if s.page==m::Page::Waypoints{m::Page::Map}else{m::Page::Waypoints}),
                        5=>m::Action::Page(if s.page==m::Page::Settings{m::Page::Map}else{m::Page::Settings}),
                        _=>m::Action::Close,
                    };
                    s.actions.push(action);return;
                }
            }
            if s.page!=m::Page::Map {
                let row=((mouse[1]-l.row(0).y)/(23.0*l.scale)).floor() as i32;
                if row<0||!l.row(row as usize).contains(mouse){return;}
                let row=row as usize;
                match s.page {
                    m::Page::Settings if row<l.rows() && row+s.page_offset<m::SPEC_COUNT=>s.actions.push(m::Action::Setting(row+s.page_offset)),
                    m::Page::Waypoints if row<l.rows()=>{
                        if let Some(p)=s.store.as_ref().and_then(|st|st.waypoints.get(row+s.page_offset)){s.actions.push(m::Action::Edit(p.id));}
                    }
                    m::Page::Editor=>{
                        if let Some(e)=s.editor.as_mut() {
                            match row {
                                0..=3=>e.focus=row,4=>e.color=(e.color+1)%m::COLORS.len(),5=>e.icon=(e.icon+1)%m::ICONS.len(),
                                6=>e.enabled=!e.enabled,7=>e.in_world=!e.in_world,8=>s.actions.push(m::Action::Save),
                                9=>{if e.confirm_delete{s.actions.push(m::Action::Delete);}else{e.confirm_delete=true;}},
                                _=>{}
                            }
                        }
                    }
                    _=>{}
                }
                return;
            }
            if l.map.contains(mouse){s.dragging=true;}
        }
        if button_index==1 && s.page==m::Page::Map && s.image_ready && s.uploaded_rect==l.map && l.map.contains(mouse) {
            // Use exactly the currently displayed transform, not a queued zoom.
            let at=s.uploaded_view.world(s.uploaded_rect,mouse);
            s.actions.push(m::Action::New(at));
        }
    });
}
pub fn scroll(game:&Game<'_>,amount:f64) {
    if !amount.is_finite()||amount==0.0{return;}
    m::with_state(|s|{
        let l=Layout::new(s.gui,s.config);
        if s.page==m::Page::Settings||s.page==m::Page::Waypoints {
            let len=if s.page==m::Page::Settings {m::SPEC_COUNT}else{s.store.as_ref().map_or(0,|st|st.waypoints.len())};
            s.page_offset=(s.page_offset as i64-if amount>0.0{1}else{-1}).clamp(0,len.saturating_sub(l.rows()) as i64) as usize;
        } else if s.page==m::Page::Map && l.map.contains(game.native_cursor()) {
            s.view.zoom(l.map,game.native_cursor(),amount);s.image_dirty=true;
        }
    });
}
pub fn screen_key(key:&str,repeat:bool,modifiers:u32) {
    m::with_state(|s|{
        if s.page==m::Page::Editor {
            if let Some(e)=s.editor.as_mut() {
                match key {
                    "key.keyboard.backspace"=>{e.fields[e.focus].pop();}
                    "key.keyboard.tab"=>{e.focus=(e.focus+if modifiers&1!=0{3}else{1})%4;}
                    "key.keyboard.a" if modifiers&2!=0=>e.fields[e.focus].clear(),
                    "key.keyboard.enter"|"key.keyboard.keypad.enter" if !repeat=>s.actions.push(m::Action::Save),
                    "key.keyboard.escape"=>{s.editor=None;s.page=m::Page::Map;}
                    _=>{}
                }
            }
            return;
        }
        if !repeat && (key==s.hotkey||key=="key.keyboard.escape"){s.actions.push(m::Action::Close);return;}
        match key {
            "key.keyboard.equal"|"key.keyboard.keypad.add"=>s.actions.push(m::Action::Zoom(1.0)),
            "key.keyboard.minus"|"key.keyboard.keypad.subtract"=>s.actions.push(m::Action::Zoom(-1.0)),
            "key.keyboard.home"=>s.actions.push(m::Action::Center),
            "key.keyboard.left"=>{s.view.pan([32.0,0.0]);s.image_dirty=true;}
            "key.keyboard.right"=>{s.view.pan([-32.0,0.0]);s.image_dirty=true;}
            "key.keyboard.up"=>{s.view.pan([0.0,32.0]);s.image_dirty=true;}
            "key.keyboard.down"=>{s.view.pan([0.0,-32.0]);s.image_dirty=true;}
            _=>{}
        }
    });
}
pub fn world_markers(game:&Game<'_>,draw:&mut crate::game::native_world_draw::NativeWorldDraw) {
    if !game.native_on(m::ID){return;}
    let Some(player)=game.native_player()else{return;};
    let client=game.native_client_state();
    if client.as_ref().is_none_or(|c|c.gui_hidden)||game.native_mod_screen().is_some(){return;}
    m::with_state(|s|{
        if !s.config.on(m::WORLD_MARKERS)||s.dimension!=player.dimension.to_string(){return;}
        if client.as_ref().and_then(|c|c.world.as_ref()).is_none_or(|w|format!("{w:?}")!=s.world){return;}
        let Some(store)=&s.store else{return;};
        for p in &store.waypoints {
            let distance=m::distance(s.player,p.pos);
            if !p.enabled||!p.in_world||distance>s.config.0[m::WORLD_RANGE] as f64 {continue;}
            let at=[p.pos[0],p.pos[1]+s.config.0[m::WORLD_HEIGHT] as f64,p.pos[2]];
            draw.push();
            if draw.translate_world(at) {
                // Face the actual camera position; the normal world renderer supplies depth testing.
                let dx=draw.camera[0]-at[0];let dz=draw.camera[2]-at[2];
                draw.rotate_about([0.0,1.0,0.0],dx.atan2(dz).to_degrees() as f32,[0.0;3]);
                let factor=0.025*s.config.0[m::WORLD_SCALE] as f32/100.0;
                draw.scale([-factor,-factor,factor]);
                let color=((s.config.0[m::WORLD_ALPHA]*255/100) as u32)<<24|p.color;
                let symbol=["◆","●","+","■"][p.icon];
                let mut label=format!("{symbol} {}",p.name);
                if s.config.on(m::DISTANCES){label.push_str(&format!(" ({distance:.0}m)"));}
                game.native_world_text(draw,&horizon_client::text::Text::literal(label),color,
                    if s.config.on(m::LABEL_BG){Some(0xc020180f)}else{None});
            }
            draw.pop();
        }
    });
}

