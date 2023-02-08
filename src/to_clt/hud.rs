use super::*;

#[mt_derive(to = "clt", repr = "u32", enumset)]
pub enum HudStyleFlag {
    Bold,
    Italic,
    Mono,
}

#[mt_derive(to = "clt", repr = "u8", tag = "attribute", content = "value")]
pub enum HudChange {
    Pos([f32; 2]) = 0,
    Name(String),
    Scale([f32; 2]),
    Text(String),
    Number(u32),
    Item(u32),
    Dir(u32),
    Align([f32; 2]),
    Offset([f32; 2]),
    WorldPos([f32; 3]),
    ZIndex(i32),
    Text2(String),
    Style(EnumSet<HudStyleFlag>),
}

#[mt_derive(to = "clt", repr = "u8")]
pub enum HudType {
    Image = 0,
    Text,
    Statbar,
    Inv,
    Waypoint,
    ImageWaypoint,
}

#[mt_derive(to = "clt")]
pub struct HudElement {
    pub hud_type: HudType,
    pub pos: [f32; 2],
    pub name: String,
    pub scale: [f32; 2],
    pub text: String,
    pub number: u32,
    pub item: u32,
    pub dir: u32,
    pub align: [f32; 2],
    pub offset: [f32; 2],
    pub world_pos: [f32; 3],
    pub z_index: i32,
    pub text_2: String,
    pub style: EnumSet<HudStyleFlag>,
}

impl HudElement {
    pub fn apply_change(&mut self, change: HudChange) {
        use HudChange::*;

        match change {
            Pos(v) => self.pos = v,
            Name(v) => self.name = v,
            Scale(v) => self.scale = v,
            Text(v) => self.text = v,
            Number(v) => self.number = v,
            Item(v) => self.item = v,
            Dir(v) => self.dir = v,
            Align(v) => self.align = v,
            Offset(v) => self.offset = v,
            WorldPos(v) => self.world_pos = v,
            ZIndex(v) => self.z_index = v,
            Text2(v) => self.text_2 = v,
            Style(v) => self.style = v,
        }
    }
}

#[mt_derive(to = "clt", repr = "u32", enumset)]
pub enum HudFlag {
    Hotbar,
    HealthBar,
    Crosshair,
    WieldedItem,
    BreathBar,
    Minimap,
    RadarMinimap,
}

#[mt_derive(to = "clt", repr = "u16", tag = "attribute", content = "value")]
pub enum HotbarParam {
    Size(#[mt(const16 = 4)] u32) = 0,
    Image(String),
    SelectionImage(String),
}

#[mt_derive(to = "clt", repr = "u16")]
pub enum MinimapType {
    None = 0,
    Surface,
    Radar,
    Texture,
}

#[mt_derive(to = "clt")]
pub struct MinimapMode {
    pub minimap_type: MinimapType,
    pub label: String,
    pub size: u16,
    pub texture: String,
    pub scale: u16,
}

#[mt_derive(to = "clt", custom)]
pub struct MinimapModePkt {
    current: u16,
    modes: Vec<MinimapMode>,
}

impl MtSerialize for MinimapModePkt {
    fn mt_serialize<C: MtCfg>(&self, writer: &mut impl Write) -> Result<(), SerializeError> {
        C::write_len(self.modes.len(), writer)?;
        self.current.mt_serialize::<DefaultCfg>(writer)?;
        for item in self.modes.iter() {
            item.mt_serialize::<DefaultCfg>(writer)?;
        }
        Ok(())
    }
}
/*
TODO: rustify

var DefaultMinimap = []MinimapMode{
    {Type: NoMinimap},
    {Type: SurfaceMinimap, Size: 256},
    {Type: SurfaceMinimap, Size: 128},
    {Type: SurfaceMinimap, Size: 64},
    {Type: RadarMinimap, Size: 512},
    {Type: RadarMinimap, Size: 256},
    {Type: RadarMinimap, Size: 128},
}
*/
