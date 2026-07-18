use super::*;

#[derive(GodotClass)]
#[class(base=Node3D)]
pub struct MinerNode {
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
