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

    #[var]
    owned: bool,
}

#[godot_api]
impl INode3D for PoleNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: PoleKey::null(),
            owned: false,
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        // add generic pole, owning machines can change the type (to consumer / generator) afterwards... 
        self.key = bound.game.add_pole();
    }

    fn exit_tree(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();
        
        bound.game.remove_pole(self.key);
        self.signals().about_to_get_removed().emit();
    }
}

#[godot_api]
impl PoleNode {
    #[signal]
    fn about_to_get_removed();
}

#[derive(GodotClass)]
#[class(base=Node3D)]
struct MachineNode {
    base: Base<Node3D>,
    key: MachineKey,
    pole_key: PoleKey,
}

#[godot_api]
impl INode3D for MachineNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: MachineKey::null(),
            pole_key: PoleKey::null(),
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        let mut _pole = self.base().get_node_as::<PoleNode>("Power Pole");
        let mut pole = _pole.bind_mut();
        pole.owned = true;
        self.pole_key = pole.key;
        
        self.key = bound.game.add_machine_with_pole(&CRUSH_IRON_RECIPE, pole.key);

        bound.game.machines[self.key].input[0].buffer = Item::full_stack(RAW_IRON_1);
    }

    fn process(&mut self, delta: f64) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        let mut label = self.base().get_node_as::<Label3D>("Label3D");
        let status = &bound.game.machines[self.key].status;
        let progress = &bound.game.machines[self.key].progress;
        let output_hatch = &bound.game.machines[self.key].output[0].buffer;
        
        label.set_text(&format!("{:?} {:?} {:?}", status, progress, output_hatch));
        //label.set_text(&format!("{:?}", bound.game.poles[self.pole_key]));
    }

    fn exit_tree(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        bound.game.remove_machine(self.key);
    }
}

#[derive(GodotClass)]
#[class(base=Node3D)]
struct GeneratorNode {
    base: Base<Node3D>,
    
    #[export]
    load: u32,
}

#[godot_api]
impl INode3D for GeneratorNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            load: 0,
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        let mut _pole = self.base().get_node_as::<PoleNode>("Power Pole");
        let mut pole = _pole.bind_mut();
        pole.owned = true;

        bound.game.add_generator_with_pole(self.load as LoadUnit, pole.key);
    }

    fn exit_tree(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();
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

        let mut pole_1 = self.pole_1_ref.clone().unwrap();
        let mut pole_1 = pole_1.bind_mut();
        let pole_1_key = pole_1.key;
        pole_1.signals().about_to_get_removed().connect_other(self, Self::pole_destroyed);

        
        let mut pole_2 = self.pole_2_ref.clone().unwrap();
        let mut pole_2 = pole_2.bind_mut();
        let pole_2_key = pole_2.key;
        pole_2.signals().about_to_get_removed().connect_other(self, Self::pole_destroyed);
        
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

#[godot_api]
impl WireNode {
    fn pole_destroyed(&mut self) {
        self.base_mut().queue_free();
    }
}