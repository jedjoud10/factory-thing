use std::collections::HashMap;
use godot::prelude::*;
use godot::classes::*;

pub mod nodes;
pub mod fluid;
pub mod handle;
pub mod items;
pub mod simulation;
pub mod tests;
mod registry;
mod heat;

pub use handle::*;
pub use items::*;
pub use simulation::*;

use crate::registry::DefaultRegistry;

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}


#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct FactoryManager {
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
        // FIXME: remove this by adding separate const settings for test cases and default cases 
        let mut sim =  Simulation::<DefaultRegistry>::default();
        sim.settings.machine_require_clicky_thing_attached = true;

        Self {
            base,
            real_time: 0f32,
            game: sim,
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

        self.item_prefab = load::<PackedScene>("res://actors/item.tscn");
    }

    fn process(&mut self, delta: f32) {
        self.real_time += delta;
    }

    fn physics_process(&mut self, _delta: f32) {
        // TODO: move to custom loop
        self.game.tick();
    }
}