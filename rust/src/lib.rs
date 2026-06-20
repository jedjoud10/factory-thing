use godot::prelude::*;

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

use godot::classes::{ISprite2D, Sprite2D};

#[derive(GodotClass)]
#[class(base=Sprite2D)]
struct SigmoidPlayer {
    speed: f64,
    angular_speed: f64,

    base: Base<Sprite2D>
}


#[godot_api]
impl ISprite2D for SigmoidPlayer {
    fn init(base: Base<Sprite2D>) -> Self {
        godot_print!("Hello, world!"); // Prints to the Godot console
        
        Self {
            speed: 400.0,
            angular_speed: std::f64::consts::PI,
            base,
        }
    }

    fn physics_process(&mut self, delta: f64) {
        // In GDScript, this would be: 
        // rotation += angular_speed * delta
        
        let radians = (self.angular_speed * delta) as f32;
        self.base_mut().rotate(radians);
        // The 'rotate' method requires a f32, 
        // therefore we convert 'self.angular_speed * delta' which is a f64 to a f32
    }
}

#[godot_api]
impl SigmoidPlayer {
    #[func]
    fn increase_speed(&mut self, amount: f64) {
        self.angular_speed += amount;
        self.signals().speed_increased().emit();
    }

    #[signal]
    fn speed_increased();
}