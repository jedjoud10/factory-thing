use slotmap::new_key_type;
pub use slotmap::Key;

new_key_type! { pub struct MachineKey; }
new_key_type! { pub struct PoleKey; }
new_key_type! { pub struct BeltKey; }
new_key_type! { pub struct WireKey; }
new_key_type! { pub struct HatchKey; }
new_key_type! { pub struct SiloKey; }
