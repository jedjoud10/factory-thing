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
pub use stuff::*;


struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}


#[derive(GodotClass)]
#[class(base=Node3D)]
struct FactoryManager {
    base: Base<Node3D>,

    game: Game
}


#[godot_api]
impl INode3D for FactoryManager {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            base,
            game: Default::default()
        }
    }

    fn physics_process(&mut self, delta: f64) {

    }
}

#[godot_api]
impl FactoryManager {
    #[func]
    fn spawn_machine(&mut self, machine_reference: Gd<Node3D>) {
        
    }

    #[func]
    fn despawn_machine(&mut self, machine_reference: Gd<Node3D>) {
    
    }
}