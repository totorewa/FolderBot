#[derive(Default)]
pub struct GameState {
    pub mainframe_password: Option<u64>,
    pub freed: Option<u64>,
    pub game_factor: u32,

    // this should go elsewhere but whatever
    pub tm_sounds: u64,
}
