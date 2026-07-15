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
    }
}

#[derive(Debug)]
struct FluidBuffer {
    amount: i32,
}

#[derive(Debug)]
struct PressurePipe {
    buffer: Vec<f32>,
}

impl PressurePipe {
    fn tick(&mut self) {
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
    }
}


#[derive(Debug)]
struct PressurePipe2 {
    left: FluidBuffer,
    right: FluidBuffer,
    flow: i32,
}

impl PressurePipe2 {
    fn tick(&mut self) {
        // pre pass
        let optimal_flow = self.left.amount - self.right.amount;
    }
}


#[test]
fn test2() {
    let mut pipe = PressurePipe { buffer: vec![0f32; 64] };
    pipe.buffer[0] = 100f32;
    
    for _ in 0..10 {
        pipe.tick();
    }
}


#[derive(Debug, Clone)]
struct TmpPart {
    direction: bool, // false = -, true = +
    position: i32,

    alpha: u32,
}


#[derive(Debug)]
struct ParticlePipe {
    particles: Vec<TmpPart>,
}

impl ParticlePipe {
    fn tick(&mut self) {
        for particle in self.particles.iter_mut() {
            // flip direction based on bounds
            if particle.direction && particle.position == 9 {
                particle.direction = false;
            }
            if !particle.direction && particle.position == 0 {
                particle.direction = true;
            }

            let offset = if particle.direction { 1 } else { -1 };
            particle.position += offset;
        }
    }

    fn volume(&self) -> u32 {
        self.particles.len() as u32
    }
}


#[test]
fn test3() {
    let mut pipe = ParticlePipe { particles: vec![] };
    
    for _ in 0..100 {
        // source
        pipe.particles.push(TmpPart { direction: true, position: 0, alpha: 0 });
        pipe.tick();
        
        // sink
        pipe.particles.retain(|x| x.position != 8);
    }
}

#[derive(Debug)]
struct CellularAutomataPipe {
    cells: Vec<i32>,
}

impl CellularAutomataPipe {
    fn tick(&mut self) {
        let previous = self.cells.clone();
        let mut deltas = self.cells.iter().map(|_| 0i32).collect::<Vec<_>>();

        for (i, cell) in previous.iter().enumerate() {
            if *cell == 0 {
                continue;
            }

            let left = (i > 0).then(|| &previous[i - 1]);
            let right = (i < previous.len() - 1).then(|| &previous[i + 1]);

            match (left, right) {
                (None, None) => {},
                (None, Some(r)) => {
                    if *cell > *r {
                        // flow from middle to right
                        deltas[i] -= 1;
                        deltas[i + 1] += 1;
                    }
                },
                (Some(l), None) => {
                    if *cell > *l {
                        // flow from middle to left
                        deltas[i] -= 1;
                        deltas[i - 1] += 1;
                    }
                },
                (Some(l), Some(r)) => {
                    if l > r || r > l {
                        if *cell > *l && r > l {
                            // flow from middle to left
                            deltas[i] -= 1;
                            deltas[i - 1] += 1;
                        }


                        if *cell > *r && l > r {
                            // flow from middle to right
                            deltas[i] -= 1;
                            deltas[i + 1] += 1;
                        }
                    } else {
                        // pick either or randomly (picks left for now)

                        deltas[i] -= 1;
                        deltas[i - 1] += 1;
                    }
                },
            }
        }

        for (i, cell) in self.cells.iter_mut().enumerate() {
            *cell += deltas[i];
        }
    }
}


#[test]
fn test4() {
    let mut pipe = CellularAutomataPipe { cells: vec![0; 10] };
    pipe.cells[0] = 1;
    
    for _ in 0..10 {
        pipe.cells[0] = 1;
        pipe.tick();
    }
}