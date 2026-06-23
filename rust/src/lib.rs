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
    real_time: f32,
    game: Game,
}


#[godot_api]
impl INode3D for FactoryManager {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            real_time: 0f32,
            game: Default::default(),
        }
    }

    fn process(&mut self, delta: f32) {
        self.real_time += delta;
    }

    fn physics_process(&mut self, delta: f32) {
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

        bound.game.machines[self.key].input[0].buffer = Item { id: RAW_IRON_1, count: 8 };

        for child in self.base().get_children().iter_shared() {
            if let Ok(mut hatch_node) = child.try_cast::<HatchNode>() {
                hatch_node.bind_mut().owning_machine_key = self.key;
            }
        }
    }

    fn process(&mut self, _delta: f32) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        let mut label = self.base().get_node_as::<Label3D>("Label3D");
        let status = &bound.game.machines[self.key].status;
        let progress = &bound.game.machines[self.key].progress;
        let output_hatch = &bound.game.machines[self.key].output[0].buffer;
        
        //label.set_text(&format!("{:?} {:?} {:?}", status, progress, output_hatch));
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
struct HatchNode {
    base: Base<Node3D>,

    #[export]
    hatch_index: u32,
    
    #[export]
    input_hatch: bool,

    owning_machine_key: MachineKey,
}

#[godot_api]
impl INode3D for HatchNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            input_hatch: false,
            hatch_index: 0,
            owning_machine_key: MachineKey::null(),
        }
    }

    
    fn process(&mut self, _delta: f32) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();

        let mut label = self.base().get_node_as::<Label3D>("Label3D");
        let machine = &bound.game.machines[self.owning_machine_key];
        
        let item: Item = if self.input_hatch {
            machine.input[self.hatch_index as usize].buffer
        } else {
            machine.output[self.hatch_index as usize].buffer
        };

        label.set_text(&item.display());
    }
}

#[derive(GodotClass)]
#[class(base=Node3D)]
struct BeltNode {
    base: Base<Node3D>,

    #[var]
    belt_start_hatch_ref: Option<Gd<HatchNode>>,
    #[var]
    belt_end_hatch_ref: Option<Gd<HatchNode>>,
    #[var]
    length: i64,

    #[var]
    start_position: Vector3,
    #[var]
    end_position: Vector3,

    items: Vec<Option<Gd<Node3D>>>,
    
    key: BeltKey,
}

#[godot_api]
impl INode3D for BeltNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            belt_start_hatch_ref: None,
            belt_end_hatch_ref: None,
            start_position: Vector3::ZERO,
            end_position: Vector3::ZERO,
            length: 0,
            items: Vec::new(),
            key: BeltKey::null()
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        let mut start_hatch = self.belt_start_hatch_ref.clone().unwrap();
        let start_hatch_machine = start_hatch.get_parent().unwrap().cast::<MachineNode>().bind().key;
        let start_hatch = start_hatch.bind_mut();


        let mut end_hatch = self.belt_end_hatch_ref.clone().unwrap();
        let end_hatch_machine = end_hatch.get_parent().unwrap().cast::<MachineNode>().bind().key;
        let end_hatch = end_hatch.bind_mut();

        let output_hatch = HatchReference { machine_index: start_hatch_machine, hatch_index: start_hatch.hatch_index as usize };
        let input_hatch = HatchReference { machine_index: end_hatch_machine, hatch_index: end_hatch.hatch_index as usize };
        self.key = bound.game.add_belt_2(output_hatch, input_hatch, self.length as usize);
    }

    fn process(&mut self, _delta: f32) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();

        // https://github.com/JL-squared/Factory-ECS-Prototype/blob/main/Assets/Actors/Systems/VisualSystems.cs#L625
        let tick_rate = 60;
        let belt = &bound.game.belts[self.key];
        let last_transfer_tick = belt.last_transfer_tick;
        let last_transfer_time = last_transfer_tick as f32 / tick_rate as f32;
        let belt_ticks_between_transfers = 16;
        let tick_interpolation_factor = (bound.real_time - last_transfer_time) * (tick_rate as f32 / belt_ticks_between_transfers as f32);
        let tick_interpolation_factor = tick_interpolation_factor.clamp(0f32, 1f32);

        let scene = load::<PackedScene>("res://item.tscn");
        let a = self.start_position;
        let b = self.end_position;

        // TODO: don't spawn / despawn entities every frame bruh
        for x in self.items.drain(..) {
            x.unwrap().free();
        }

        for (i, elem) in belt.buffer.iter().enumerate() {
            let factor_1 = i as f32 / (belt.buffer.len() as f32);
            let factor_2 = (i+1) as f32 / (belt.buffer.len() as f32);
            let factor = f32::lerp(factor_1, factor_2, tick_interpolation_factor);
            
            
            if !elem.is_invalid() {
                let instance = scene.instantiate().unwrap();
                self.base_mut().add_child(&instance);
                
                let mut instance = instance.cast::<Node3D>();
                let lerped = Vector3::lerp(a, b, factor);
                
                instance.set_global_position(lerped);
                self.items.push(Some(instance));
            }
        }
    }

    fn exit_tree(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        bound.game.remove_belt(self.key);
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