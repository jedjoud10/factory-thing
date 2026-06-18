use std::collections::HashMap;

#[derive(Debug)]
struct Particle {
    position: i32,
    velocity: i32,
}

#[derive(Debug)]
struct Pipe {
    //buffer: Vec<f32>,
    particles: Vec<Particle>
}

impl Pipe {
    fn tick(&mut self) {
        for particle in self.particles.iter_mut() {
            particle.position += particle.velocity;
        }

        /*
        for index1 in 0..self.particles.len() {
            for index2 in 0..self.particles.len() {
                if index1 != index2 {
                    let distance = self.particles[index1].position - self.particles[index2].position;

                    if distance != 0f32 {
                        let force = 1f32 / distance;
                        self.particles[index1].velocity += force * 0.5f32;
                    }
                }
            }
        }

        for index1 in 0..self.particles.len() {
            for index2 in 0..self.particles.len() {
                if index1 != index2 {
                    let distance = self.particles[index1].position - self.particles[index2].position;

                    if distance.abs() < 1f32 {
                        let other = self.particles[index2].velocity;
                        let myself = self.particles[index1].velocity;

                        self.particles[index1].velocity = (other + myself) * 0.5f32;
                        self.particles[index2].velocity = (other + myself) * 0.5f32;
                    }
                }
            }
        }

        for particle in self.particles.iter_mut() {
            if particle.position < 0f32 {
                particle.position = 0f32;
                particle.velocity = 5f32;
            }

            if particle.position >= 32f32 {
                particle.position = 31f32;
                particle.velocity = -5f32;
            }
        }
        */



        /*
        let prev = &mut self.buffer;
        dbg!(&prev);
        
        for pass in 0..8 {
            let mut copy = prev.to_vec();
            for index  in 1..(copy.len()-1) {
                let left = prev[index - 1];
                let right = prev[index + 1];
                let middle = prev[index];

                let c = 0.5f32;

                let left_to_middle = (left - middle).max(0f32);
                copy[index - 1] -= left_to_middle * c;
                copy[index] += left_to_middle * c;
            
            
                let right_to_middle = (right - middle).max(0f32);
                copy[index + 1] -= right_to_middle * c;
                copy[index] += right_to_middle * c;
            } 

            prev.copy_from_slice(&copy);
        }
        */


    }

    fn volume(&self) -> Vec<f32> {
        let mut test = vec![0f32; 32];

        for particle in self.particles.iter() {
            if particle.position >= 0 && particle.position < 32 {
                test[particle.position as usize] += 1.0f32;
            } else {
                dbg!(particle.position);
            }
        }

        test
    }

    fn spawn(&mut self) {
        for i in 0..1 {
            self.particles.push(Particle {
                position: 0i32, // spawn them "randomly" near 0-1m
                velocity: 1i32,
            });
        }
    }
}

#[test]
fn test() {
    let mut pipe = Pipe { particles: vec![] };
    
    
    for _ in 0..10 {
        pipe.spawn();
        pipe.tick();
        dbg!(pipe.volume());
    }
}