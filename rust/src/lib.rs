use godot::prelude::*;
use godot::classes::*;

pub mod fluid;
pub mod handle;
pub mod items;
pub mod stuff;
pub mod tests;

pub use fluid::*;
pub use handle::*;
pub use items::*;
use slotmap::Key;
use slotmap::KeyData;
use slotmap::SecondaryMap;
pub use stuff::*;


struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}


#[derive(GodotClass)]
#[class(base=Node3D)]
struct FactoryManager {
    base: Base<Node3D>,

    game: Game,
}


#[godot_api]
impl INode3D for FactoryManager {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            game: Default::default(),
        }
    }

    fn physics_process(&mut self, delta: f64) {
        // TODO: move to custom loop
        self.game.tick();
    }
}


#[derive(GodotClass)]
#[class(base=Node3D)]
struct PoleNode {
    base: Base<Node3D>,
    key: PoleKey,
}

#[godot_api]
impl INode3D for PoleNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: PoleKey::null()
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        // add generic pole, owning machines can change the type (to consumer / generator) afterwards... 
        let pole_key = bound.game.add_pole();
        self.key = pole_key;
    }

    fn exit_tree(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();
        bound.game.remove_pole(self.key);
    }
}

#[derive(GodotClass)]
#[class(base=Node3D)]
struct MachineNode {
    base: Base<Node3D>,
    key: MachineKey,
}

#[godot_api]
impl INode3D for MachineNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: MachineKey::null()
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        let _pole = self.base().get_node_as::<PoleNode>("Power Pole");
        let pole = _pole.bind();

        
        let machine_key = bound.game.add_machine_with_pole(&CRUSH_IRON_RECIPE, pole.key);
        self.key = machine_key;
    }

    fn exit_tree(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        // TODO: need to disconnect every wire that is connected to this pole
        // TODO: do the same but for the server. this can avoid doing a roundtrip to the client and back
    }
}

#[derive(GodotClass)]
#[class(base=Node3D)]
struct WireNode {
    base: Base<Node3D>,
    key: WireKey,

    #[var]
    pole_1_ref: Option<Gd<PoleNode>>,
    #[var]
    pole_2_ref: Option<Gd<PoleNode>>,
}

#[godot_api]
impl INode3D for WireNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: WireKey::null(),
            pole_1_ref: None,
            pole_2_ref: None,
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        let pole_1_key = self.pole_1_ref.as_ref().unwrap().bind().key;
        let pole_2_key = self.pole_2_ref.as_ref().unwrap().bind().key;

        self.key = bound.game.add_wire(pole_1_key, pole_2_key);
    }

    fn exit_tree(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        bound.game.remove_wire(self.key);
    }
}