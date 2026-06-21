use std::ops::Deref;
use slotmap::new_key_type;


new_key_type! { pub struct MachineKey; }
new_key_type! { pub struct PoleKey; }
new_key_type! { pub struct BeltKey; }
new_key_type! { pub struct WireKey; }