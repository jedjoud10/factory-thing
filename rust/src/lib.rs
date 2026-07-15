use std::collections::HashMap;

use godot::prelude::*;
use godot::classes::*;

pub mod fluid;
pub mod handle;
pub mod items;
pub mod simulation;
pub mod tests;
mod registry;
mod heat;

pub use fluid::*;
pub use handle::*;
pub use items::*;
use slotmap::Key;
use slotmap::KeyData;
use slotmap::SecondaryMap;
pub use simulation::*;

use crate::registry::DefaultRegistry;
use crate::registry::Registry;


struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}


#[derive(GodotClass)]
#[class(base=Node3D)]
struct FactoryManager {
    base: Base<Node3D>,
    real_time: f32,
    game: Simulation<DefaultRegistry>,
    godot_item_models_registry: HashMap<u8, Gd<PackedScene>>,
    godot_item_materials_registry: HashMap<u8, Gd<Material>>,
    item_prefab: Gd<PackedScene>,
}


#[godot_api]
impl INode3D for FactoryManager {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            real_time: 0f32,
            game: Default::default(),
            godot_item_models_registry: HashMap::default(),
            godot_item_materials_registry: HashMap::default(),
            item_prefab: Gd::default()
        }
    }

    fn enter_tree(&mut self) {
        for (index, item) in DefaultRegistry::ITEMS.iter().enumerate() {
            if index != 0 {
                self.godot_item_models_registry.insert(index as u8, load::<PackedScene>(item.data.item_model_resource));
                self.godot_item_materials_registry.insert(index as u8, load::<Material>(item.data.item_material_resource));
            }
        }

        self.item_prefab = load::<PackedScene>("res://item.tscn");
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
    }
}

#[godot_api]
impl PoleNode {}

#[derive(GodotClass)]
#[class(base=Node3D)]
struct MachineNode {
    base: Base<Node3D>,
    key: MachineKey,
    pole_key: PoleKey,

    #[export]
    recipe_name_id: godot::builtin::GString,
}

#[godot_api]
impl INode3D for MachineNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: MachineKey::null(),
            pole_key: PoleKey::null(),
            recipe_name_id: godot::builtin::GString::default(),
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

        let id = self.recipe_name_id.to_string();
        let x = DefaultRegistry::registry_recipe(&id);
        
        self.key = bound.game.add_machine_with_pole(&x, pole.key);

        // *bound.game.get_input_hatch_mut(self.key, 0) = Item { id: DefaultRegistry::RAW_IRON_1, count: 8 };

        for child in self.base().get_children().iter_shared() {
            if let Ok(mut hatch_node) = child.try_cast::<HatchNode>() {
                let mut node = hatch_node.bind_mut();

                // update the HatchNode key based on the generated hatch keys when inserting into the manager
                if node.input_hatch {
                    node.key = bound.game.machines[self.key].input[node.hatch_index as usize];
                } else {
                    node.key = bound.game.machines[self.key].output[node.hatch_index as usize];
                }
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
        // let output_hatch = &bound.game.machines[self.key].output[0].buffer;
        
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
struct MinerNode {
    base: Base<Node3D>,
    key: MachineKey,
    pole_key: PoleKey,

    #[export]
    recipe_name_id: godot::builtin::GString,
}

#[godot_api]
impl INode3D for MinerNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: MachineKey::null(),
            pole_key: PoleKey::null(),
            recipe_name_id: godot::builtin::GString::default(),
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

        let id = self.recipe_name_id.to_string();
        let x = DefaultRegistry::registry_recipe(&id);
        
        self.key = bound.game.add_miner_with_pole(&x, pole.key);

        // *bound.game.get_input_hatch_mut(self.key, 0) = Item { id: DefaultRegistry::RAW_IRON_1, count: 8 };

        for child in self.base().get_children().iter_shared() {
            if let Ok(mut hatch_node) = child.try_cast::<HatchNode>() {
                let mut node = hatch_node.bind_mut();

                // update the HatchNode key based on the generated hatch keys when inserting into the manager
                if node.input_hatch {
                    node.key = bound.game.machines[self.key].input[node.hatch_index as usize];
                } else {
                    node.key = bound.game.machines[self.key].output[node.hatch_index as usize];
                }
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
        // let output_hatch = &bound.game.machines[self.key].output[0].buffer;
        
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
struct SiloNode {
    base: Base<Node3D>,
    key: SiloKey,
    
    #[export]
    starting_item_id: u32,

    #[export]
    starting_item_count: u32,

    #[export]
    starting_item_stacks: u32,
}

#[godot_api]
impl INode3D for SiloNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            key: SiloKey::null(),
            starting_item_id: 0,
            starting_item_count: 0,
            starting_item_stacks: 0,
        }
    }

    fn ready(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        self.key = bound.game.add_silo();

        bound.game.silos[self.key].stack.extend(std::iter::repeat(Item::new(self.starting_item_id as u8, self.starting_item_count as u8)).take(self.starting_item_stacks as usize));

        for child in self.base().get_children().iter_shared() {
            if let Ok(mut hatch_node) = child.try_cast::<HatchNode>() {
                let mut node = hatch_node.bind_mut();

                // update the HatchNode key based on the generated hatch keys when inserting into the manager
                if node.input_hatch {
                    node.key = bound.game.silos[self.key].input;
                } else {
                    node.key = bound.game.silos[self.key].output;
                }
            }
        }
    }

    fn process(&mut self, _delta: f32) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();

        let mut label = self.base().get_node_as::<Label3D>("Label3D");
        let silo = &bound.game.silos[self.key];
        let mut total = HashMap::<u8, u32>::new();

        for stack in silo.stack.iter() {
            *total.entry(stack.id).or_default() += stack.count as u32;
        }

        let mut str = String::new();
        for (id, count) in total {
            let name = DefaultRegistry::name(id);
            str += &format!("{name} x {count}\n");
        }
        label.set_text(&str);
    }

    fn exit_tree(&mut self) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let mut factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let mut bound = factory_manager.bind_mut();

        bound.game.remove_silo(self.key);
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

    key: HatchKey,
}

#[godot_api]
impl INode3D for HatchNode {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            input_hatch: false,
            hatch_index: 0,
            key: HatchKey::null()
        }
    }

    
    fn process(&mut self, _delta: f32) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();

        let mut label = self.base().get_node_as::<Label3D>("Label3D");
        let item = bound.game.hatches[self.key].buffer;
        
        label.set_text(&item.display::<DefaultRegistry>());
    }
}

#[godot_api]
impl HatchNode {}

#[derive(GodotClass)]
#[class(base=Node3D)]
struct BeltNode {
    base: Base<Node3D>,

    #[export]
    belt_start_hatch_ref: Option<Gd<HatchNode>>,
    #[export]
    belt_end_hatch_ref: Option<Gd<HatchNode>>,

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

        let start_hatch = self.belt_start_hatch_ref.clone().unwrap();
        start_hatch.signals().tree_exiting().connect_other(self, Self::hatch_destroyed);
        let start_hatch = start_hatch.bind();


        let end_hatch = self.belt_end_hatch_ref.clone().unwrap();
        end_hatch.signals().tree_exiting().connect_other(self, Self::hatch_destroyed);
        let end_hatch = end_hatch.bind();

        let input_hatch = end_hatch.key;
        let output_hatch = start_hatch.key;
        
        let a = start_hatch.base().get_global_position();
        let b = end_hatch.base().get_global_position();
        let length = Vector3::distance_to(a,b);

        self.key = bound.game.add_belt_2(output_hatch, input_hatch, BeltSize::WorldLength(length));
        godot::global::godot_print!("{:?}", self.key);
    }

    
    // TODO: because some physics tick can be skipped (when running in sped-up mode, for example), this can cause a desync between world time and tick time
    // easiest fix is to get this to run on a tick basis instead. this *will* give chopped animations...
    fn physics_process(&mut self, _delta: f32) {
        let tree = self.base().get_tree();
        let window = tree.get_root().unwrap();
        let root = window.get_child(0).unwrap();
        let factory_manager = root.get_node_as::<FactoryManager>("FactoryManager");
        let bound = factory_manager.bind();

        // https://github.com/JL-squared/Factory-ECS-Prototype/blob/main/Assets/Actors/Systems/VisualSystems.cs#L625
        let belt = &bound.game.belts[self.key];
        let last_transfer_tick = belt.last_transfer_tick;

        let belt_ticks_between_transfers = bound.game.settings.belt_ticks_between_transfers;
        let diff = (bound.game.tick - last_transfer_tick) as f32 / belt_ticks_between_transfers as f32;

        let tick_interpolation_factor = diff;
        let tick_interpolation_factor = tick_interpolation_factor.clamp(0f32, 1f32);

        let a = self.belt_start_hatch_ref.as_ref().unwrap().bind().base().get_global_position();
        let b = self.belt_end_hatch_ref.as_ref().unwrap().bind().base().get_global_position();

        // TODO: don't spawn / despawn entities every frame bruh
        for x in self.items.drain(..) {
            x.unwrap().free();
        }

        for (i, elem) in belt.buffer.iter().enumerate() {
            let factor_1 = i as f32 / (belt.buffer.len() as f32);
            let factor_2 = (i+1) as f32 / (belt.buffer.len() as f32);
            let factor = f32::lerp(factor_1, factor_2, tick_interpolation_factor);
            
            
            if !elem.is_invalid() {
                let mut item_node = bound.item_prefab.instantiate().unwrap();


                let model_node = bound.godot_item_models_registry[&elem.id].instantiate().unwrap();
                let material = &bound.godot_item_materials_registry[&elem.id];
                
                
                for child in model_node.get_children().iter_shared() {
                    if let Ok(mut mesh_instance) = child.try_cast::<MeshInstance3D>() {
                        mesh_instance.set_material_override(material);
                    }
                }



                item_node.add_child(&model_node);

                self.base_mut().add_child(&item_node);
                
                let mut instance = item_node.cast::<Node3D>();
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

#[godot_api]
impl BeltNode {
    fn hatch_destroyed(&mut self) {
        self.base_mut().queue_free();
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
}

#[derive(GodotClass)]
#[class(base=Node3D)]
struct WireNode {
    base: Base<Node3D>,
    key: WireKey,

    #[export]
    pole_1_ref: Option<Gd<PoleNode>>,
    #[export]
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
        pole_1.signals().tree_exiting().connect_other(self, Self::pole_destroyed);

        
        let mut pole_2 = self.pole_2_ref.clone().unwrap();
        let mut pole_2 = pole_2.bind_mut();
        let pole_2_key = pole_2.key;
        pole_2.signals().tree_exiting().connect_other(self, Self::pole_destroyed);
        
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