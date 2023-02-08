use crate::*;

#[mt_derive(to = "clt")]
pub struct ArgbColor {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[mt_derive(to = "clt", repr = "u8")]
pub enum ModChanSig {
    JoinOk = 0,
    JoinFail,
    LeaveOk,
    LeaveFail,
    NotRegistered,
    SetState,
}

mod chat;
mod env;
mod hud;
mod media;
mod status;

pub use chat::*;
pub use env::*;
pub use hud::*;
pub use media::*;
pub use status::*;

#[mt_derive(to = "clt", repr = "u8", tag = "type", content = "data")]
pub enum ToCltPkt {
    Hello {
        serialize_version: u8,
        #[mt(const16 = 1)] // compression
        proto_version: u16,
        auth_methods: EnumSet<AuthMethod>,
        username: String,
    } = 2,
    AcceptAuth {
        player_pos: [f32; 3],
        map_seed: u64,
        send_interval: f32,
        sudo_auth_methods: EnumSet<AuthMethod>,
    } = 3,
    AcceptSudoMode {
        sudo_auth_methods: EnumSet<AuthMethod>,
    } = 4,
    DenySudoMode = 5,
    Kick(KickReason) = 10,
    BlockData {
        pos: [i16; 3],
        #[mt(zstd)]
        block: Box<MapBlock>,
    } = 32,
    AddNode {
        pos: [i16; 3],
        param0: u16,
        param1: u8,
        param2: u8,
        keep_meta: bool,
    } = 33,
    RemoveNode {
        pos: [i16; 3],
    } = 34,
    Inv {
        inv: String,
    } = 39,
    TimeOfDay {
        time: u16,
        speed: f32,
    } = 41,
    CsmRestrictionFlags {
        flags: EnumSet<CsmRestrictionFlag>,
        map_range: u32,
    } = 42,
    AddPlayerVelocity {
        vel: [f32; 3],
    } = 43,
    MediaPush {
        no_len_hash: String,
        filename: String,
        callback_token: u32,
        should_cache: bool,
    } = 44,
    ChatMsg {
        #[mt(const8 = 1)]
        msg_type: ChatMsgType,
        #[mt(utf16)]
        sender: String,
        #[mt(utf16)]
        text: String,
        timestamp: i64, // unix time
    } = 47,
    ObjRemoveAdd {
        remove: Vec<u16>,
        add: Vec<ObjAdd>,
    } = 49,
    ObjMsgs {
        msgs: Vec<ObjMsg>,
    } = 50,
    Hp {
        hp: u16,
        #[mt(default)]
        damage_effect: bool,
    } = 51,
    MovePlayer {
        pos: [f32; 3],
        pitch: f32,
        yaw: f32,
    } = 52,
    LegacyKick {
        #[mt(utf16)]
        reason: String,
    } = 53,
    Fov {
        fov: f32,
        multiplier: bool,
        transition_time: f32,
    } = 54,
    DeathScreen {
        point_cam: bool,
        point_at: [f32; 3],
    } = 55,
    Media {
        n: u16,
        i: u16,
        files: Vec<MediaPayload>, // FIXME: can we use a HashMap for this?
    } = 56,
    NodeDefs {
        defs: Vec<NodeDef>,
    } = 58,
    AnnounceMedia {
        files: Vec<MediaAnnounce>, // FIXME: can we use a HashMap for this?
        url: String,
    } = 60,
    #[mt(size32, zlib)]
    ItemDefs {
        #[mt(const8 = 0)] // version
        defs: Vec<ItemDef>,
        aliases: HashMap<String, String>,
    } = 61,
    PlaySound {
        id: u32,
        name: String,
        gain: f32,
        src_type: SoundSrcType,
        pos: [f32; 3],
        src_obj_id: u16,
        #[serde(rename = "loop")]
        sound_loop: bool,
        fade: f32,
        pitch: f32,
        ephermeral: bool,
    } = 63,
    StopSound {
        id: u32,
    } = 64,
    Privs {
        privs: HashSet<String>,
    } = 65,
    InvFormspec {
        #[mt(size32)]
        formspec: String,
    } = 66,
    DetachedInv {
        name: String,
        keep: bool,
        len: u16,
        #[mt(len0)]
        inv: String,
    } = 67,
    ShowFormspec {
        #[mt(len32)]
        formspec: String,
        formname: String,
    } = 68,
    Movement {
        default_accel: f32,
        air_accel: f32,
        fast_accel: f32,
        walk_speed: f32,
        crouch_speed: f32,
        fast_speed: f32,
        climb_speed: f32,
        jump_speed: f32,
        gravity: f32,
    } = 69,
    SpawnParticle {
        pos: [f32; 3],
        vel: [f32; 3],
        acc: [f32; 3],
        expiration_time: f32,
        size: f32,
        collide: bool,
        #[mt(len32)]
        texture: String,
        vertical: bool,
        collision_rm: bool,
        anim_params: TileAnim,
        glow: u8,
        obj_collision: bool,
        node_param0: u16,
        node_param2: u8,
        node_tile: u8,
    } = 70,
    AddParticleSpawner {
        amount: u16,
        duration: f32,
        pos: [[f32; 3]; 2],
        vel: [[f32; 3]; 2],
        acc: [[f32; 3]; 2],
        expiration_time: [f32; 2],
        size: [f32; 2],
        collide: bool,
        #[mt(len32)]
        texture: String,
        id: u32,
        vertical: bool,
        collision_rm: bool,
        attached_obj_id: u16,
        anim_params: TileAnim,
        glow: u8,
        obj_collision: bool,
        node_param0: u16,
        node_param2: u8,
        node_tile: u8,
    } = 71,
    AddHud {
        id: u32,
        hud: HudElement,
    } = 73,
    RemoveHud {
        id: u32,
    } = 74,
    ChangeHud {
        id: u32,
        change: HudChange,
    } = 75,
    HudFlags {
        flags: EnumSet<HudFlag>,
        mask: EnumSet<HudFlag>,
    } = 76,
    SetHotbarParam(HotbarParam) = 77,
    Breath {
        breath: u16,
    } = 78,
    // TODO
    SkyParams = 79,
    OverrideDayNightRatio {
        #[serde(rename = "override")]
        ratio_override: bool,
        ratio: u16,
    } = 80,
    LocalPlayerAnim {
        idle: [i32; 2],
        walk: [i32; 2],
        dig: [i32; 2],
        walk_dig: [i32; 2],
        speed: f32,
    } = 81,
    EyeOffset {
        first: [f32; 3],
        third: [f32; 3],
    } = 82,
    RemoveParticleSpawner {
        id: u32,
    } = 83,
    CloudParams {
        density: f32,
        diffuse_color: ArgbColor,
        ambient_color: ArgbColor,
        height: f32,
        thickness: f32,
        speed: [f32; 2],
    } = 84,
    FadeSound {
        id: u32,
        step: f32,
        gain: f32,
    } = 85,
    UpdatePlayerList {
        update_type: PlayerListUpdateType,
        players: HashSet<String>,
    } = 86,
    ModChanMsg {
        channel: String,
        sender: String,
        msg: String,
    } = 87,
    ModChanSig {
        signal: ModChanSig,
        channel: String,
    } = 88,
    NodeMetasChanged(#[mt(size32)] HashMap<[i16; 3], NodeMeta>) = 89,
    SunParams {
        visible: bool,
        texture: String,
        tone_map: String,
        rise: String,
        rising: bool,
        size: f32,
    } = 90,
    MoonParams {
        visible: bool,
        texture: String,
        tone_map: String,
        size: f32,
    } = 91,
    StarParams {
        visible: bool,
        texture: String,
        tone_map: String,
        size: f32,
    } = 92,
    SrpBytesSaltB {
        salt: Vec<u8>,
        b: Vec<u8>,
    } = 96,
    FormspecPrepend {
        prepend: String,
    } = 97,
    MinimapModes(MinimapModePkt) = 98,
}
