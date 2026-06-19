use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZeroU16,
    ops::{AddAssign, Deref, DerefMut, Sub},
    sync::atomic::AtomicI32,
};

mod fluid;
mod handle;
use handle::*;

#[derive(PartialEq, Eq, Debug)]
struct RegistryItem {
    name: &'static str,
    stack_size: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct Item {
    id: u8,
    count: u8,
}

impl Item {
    const fn one(id: u8) -> Self {
        Self { id, count: 1 }
    }

    const fn full_stack(id: u8) -> Self {
        Self { id, count: REGISTRY[id as usize].stack_size }
    }

    const fn new(id: u8, count: u8) -> Self {
        Self { id, count }
    }

    const fn invalid() -> Self {
        Self { id: 0, count: 0 }
    }

    const fn is_invalid(&self) -> bool {
        self.id == 0 && self.count == 0
    }

    const fn invalidate(&mut self) {
        *self = Self::invalid();
    }

    const fn accumulate(&mut self, other: &Item) {
        assert!(self.is_invalid() || self.id == other.id);

        self.id = other.id;
        self.count += other.count;

        assert!(self.count <= REGISTRY[self.id as usize].stack_size);
    }

    const fn take(&mut self, other: &Item) {
        assert!(self.id == other.id);
        self.count = self.count.saturating_sub(other.count);

        if self.count == 0 {
            self.invalidate();
        }
    }
}

#[derive(Debug)]
struct HatchReference {
    machine_index: usize,

    // we inherently know if this is an Input or Output hatch since the Belt stores the belt_start and belt_end information
    // ex: belt_end could only refer to input hatches since that's the only hatch direction they can connect to
    hatch_index: usize,
}

#[derive(Debug)]
struct Belt {
    belt_start: HatchReference,
    belt_end: HatchReference,
    buffer: Vec<Item>,
}

#[derive(Debug)]
struct Hatch {
    buffer: Item,
}

impl Hatch {
    const fn item(id: u8, count: u8) -> Self {
        Self {
            buffer: Item { id, count },
        }
    }

    const fn empty() -> Self {
        Self {
            buffer: Item::invalid(),
        }
    }
}

#[derive(Debug)]
struct Progress {
    ticks_remaining: NonZeroU16,
    
    // used if the machine is being underpowered / inefficient
    slow_down_ticks_remaining: Option<NonZeroU16>,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum MachineStatus {
    RecipeInputResourcesMismatchOrEmpty,
    RecipeOutputResourcesMismatchOrFull,
    
    Underpowered,
    NonPowered,

    #[default]
    None,
}

#[derive(Default, Debug)]
struct Machine {
    input: Vec<Hatch>,
    output: Vec<Hatch>,
    recipe: Option<&'static Recipe>,
    progress: Option<Progress>,
    status: MachineStatus,
    pole: Option<PoleId>,
}

const RAW_IRON_1: u8 = 1;

const CRUSHED_IRON: u8 = 2;

const IRON_DUST: u8 = 3;

const IRON_INGOT: u8 = 4;

const REGISTRY: &'static [RegistryItem] = &[
    RegistryItem {
        name: "invalid",
        stack_size: 0,
    },
    RegistryItem {
        name: "Raw Iron",
        stack_size: 255,
    },
    RegistryItem {
        name: "Crushed Iron",
        stack_size: 255,
    },
    RegistryItem {
        name: "Iron Dust",
        stack_size: 255,
    },
    RegistryItem {
        name: "Iron Ingot",
        stack_size: 255,
    },
];

#[derive(Debug)]
struct Recipe {
    input: &'static [Item],
    output: &'static [Item],
    ticks: u16,
    load: LoadUnit,
}

const CRUSH_IRON_RECIPE: Recipe = Recipe {
    input: &[Item::one(RAW_IRON_1)],
    output: &[Item::one(CRUSHED_IRON)],
    ticks: 16,
    load: 10,
};

const CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE: Recipe = Recipe {
    input: &[Item::new(RAW_IRON_1, 4)],
    output: &[Item::new(CRUSHED_IRON, 4)],
    ticks: 16,
    load: 10,
};

const CRUSH_IRON_MORE_RECIPE: Recipe = Recipe {
    input: &[Item::one(CRUSHED_IRON)],
    output: &[Item::one(IRON_DUST)],
    ticks: 16,
    load: 0,
};

const SMELT_IRON_RECIPE: Recipe = Recipe {
    input: &[Item::one(IRON_DUST)],
    output: &[Item::one(IRON_INGOT)],
    ticks: 4,
    load: 0,
};

type LoadUnit = isize;

#[derive(Debug)]
enum Pole {
    // generator pole
    Generator {
        max_load: LoadUnit,
        current_load: LoadUnit,
    },

    // consumer pole
    Consumer {
        target_load: LoadUnit,
        current_load: LoadUnit,
    },

    // non powered pole
    Other,
}

#[derive(Debug)]
struct Wire {
    a: PoleId,
    b: PoleId,
    flow: LoadUnit,
}

#[derive(Default)]
struct Game {
    machines: Vec<Machine>,
    belts: Vec<Belt>,
    poles: Vec<Pole>,
    wires: Vec<Wire>,
}

impl Game {
    fn tick(&mut self) {
        let machines = &mut self.machines;
        let belts = &mut self.belts;
        let poles = &mut self.poles;
        let wires = &mut self.wires;

        // create adjacency map that stores neighbouring poles
        let mut lookup = HashMap::<PoleId, Vec<(PoleId, WireId)>>::new();
        for (wire_index, Wire { a, b, .. }) in wires.iter().enumerate() {
            lookup
                .entry(*a)
                .or_default()
                .push((*b, WireId::from_raw(wire_index)));
            lookup
                .entry(*b)
                .or_default()
                .push((*a, WireId::from_raw(wire_index)));
        }

        // reset load of poles
        for pole in poles.iter_mut() {
            match pole {
                Pole::Generator { current_load, .. } => {
                    *current_load = 0;
                }
                Pole::Consumer { current_load, .. } => {
                    *current_load = 0;
                }
                Pole::Other => {}
            }
        }

        // reset wire flow
        for wire in wires.iter_mut() {
            wire.flow = 0;
        }

        // get all pole consumers
        let consumers = poles
            .iter()
            .enumerate()
            .filter_map(|(index, pole)| {
                if let Pole::Consumer { target_load, .. } = pole {
                    Some((PoleId::from_raw(index), *target_load))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for (consumer_index, consumer_target_load) in consumers {
            assert!(consumer_target_load >= 0);
            let mut consumer_current_load = 0;

            let mut backtracking = HashMap::<PoleId, (PoleId, WireId)>::new();
            let mut generators_used = HashSet::<(PoleId, LoadUnit)>::new();
            let mut visited = HashSet::<PoleId>::new();

            let mut queue = VecDeque::<PoleId>::new();
            queue.push_back(consumer_index);

            // simple BFS shortest-path search to find enough load to satisfy consumer
            while let Some(index) = queue.pop_front() {
                let neighbours = &lookup[&index];

                for (neighbour_index, wire_index) in neighbours {
                    let consumer_remaining_load_to_satisfy =
                        consumer_target_load - consumer_current_load;

                    assert!(consumer_remaining_load_to_satisfy >= 0);

                    let neighbour = &mut poles[**neighbour_index];

                    if visited.insert(*neighbour_index) {
                        match neighbour {
                            Pole::Generator {
                                max_load,
                                current_load: current_generator_load,
                            } => {
                                // calculate the generator's remaining load
                                let generator_remaining_load = *max_load - *current_generator_load;

                                // calculcate how much load the consumer should take off of that
                                let consumer_taken_load = generator_remaining_load
                                    .min(consumer_remaining_load_to_satisfy);

                                // add load to consumer
                                consumer_current_load += consumer_taken_load;

                                // add load to generator
                                *current_generator_load += consumer_taken_load;

                                backtracking.insert(*neighbour_index, (index, *wire_index));
                                generators_used.insert((*neighbour_index, consumer_taken_load));
                            }
                            Pole::Consumer { .. } => {}
                            Pole::Other => {
                                queue.push_back(*neighbour_index);
                                backtracking.insert(*neighbour_index, (index, *wire_index));
                            }
                        }
                    }
                }
            }

            match &mut poles[*consumer_index] {
                Pole::Consumer { current_load, .. } => {
                    *current_load = consumer_current_load;
                }
                _ => unreachable!(),
            }

            // starting from the used generators, backtrack towards the consumer and modify wire flow along the way
            for (generator_used, load) in generators_used {
                // starts at generator
                let mut opt_pole = Some(generator_used);

                // `pole` is the pole closer to the generator
                while let Some(pole) = opt_pole.take() {
                    // eventually this will reach the consumer itself
                    // `new_pole_id` is the pole closer to the consumer
                    if let Some((new_pole_id, wire_index)) = backtracking.get(&pole).copied() {
                        opt_pole = Some(new_pole_id);

                        let wire = &mut wires[*wire_index];
                        if wire.a == pole && wire.b == new_pole_id {
                            // `a` is closer to gen
                            // `b` is closer to con
                            wires[*wire_index].flow += load; // flow from `a` to `b` is positive
                        } else if wire.b == pole && wire.a == new_pole_id {
                            // `a` is closer to con
                            // `b` is closer to gen
                            wires[*wire_index].flow -= load; // flow from `a` to `b` is negative
                        } else {
                            unreachable!();
                        }
                    }
                }
            }
        }

        for machine in machines.iter_mut() {
            let reset_progress = if let Some(progress) = machine.progress.as_mut() {
                let satisfied_and_target_load =
                    machine.pole.map(|pole_id| match self.poles[*pole_id] {
                        Pole::Consumer {
                            target_load,
                            current_load,
                        } => (current_load, target_load),
                        _ => unreachable!(),
                    });

                dbg!(satisfied_and_target_load);

                if let Some((satisfied, target)) = satisfied_and_target_load {
                    if satisfied == 0 {
                        // "infinite" slow down ticks for machines that are not powered at all
                        progress.slow_down_ticks_remaining = NonZeroU16::new(u16::MAX);
                        machine.status = MachineStatus::NonPowered;
                    } else if satisfied < target {
                        machine.status = MachineStatus::Underpowered;

                        // can only update once this has been reset (in case fluctuating power)
                        if progress.slow_down_ticks_remaining.is_none() {
                            // calculate efficiency percentage
                            let percent = satisfied as f32 / target as f32;

                            // 100% result in 1 ticks (which will get reset immediately after it gets set)
                            // 50% result in 2 ticks
                            // 25% result in 4 ticks
                            let inv = (1.0f32 / percent) as u16;
                            progress.slow_down_ticks_remaining = NonZeroU16::new(inv);
                        }
                    }
                } else {
                    // no slow down ticks for machines without poles
                    progress.slow_down_ticks_remaining = None;
                }
                
                // prioritize slow down ticks first
                let progress_normally = if let Some(slow_down_ticks) = progress.slow_down_ticks_remaining {
                    // special case: non powered machines 
                    if slow_down_ticks.get() == u16::MAX {
                        false
                    } else {
                        // if the decremented slow_down_ticks_remaning is zero, then it will result in None (which works in our favour)
                        progress.slow_down_ticks_remaining = NonZeroU16::new(slow_down_ticks.get() - 1);

                        // when this is none, then we have progressed through all slowdown ticks
                        progress.slow_down_ticks_remaining.is_none()
                    }
                } else {
                    true
                };

                if progress_normally {
                    // machine is currently progressing through the recipe, take one tick off
                    // TODO: add pause / stop / resume functionality here
                    let non_zero = NonZeroU16::new(progress.ticks_remaining.get() - 1);

                    if let Some(non_zero) = non_zero {
                        // number of ticks is non-zero, update, and continue
                        progress.ticks_remaining = non_zero;
                        false
                    } else {
                        // machine finished the recipe (remaining ticks is zero, but no need to update it, as we invalidate `progress` anyways)
                        // `unwrap` here is safe because the `recipe: Option<&'static Recipe>` should not be set to `None` when a machine is progressing
                        let recipe = &machine.recipe.unwrap();

                        // take items from input hatches
                        for (recipe_input, hatch_input) in
                            recipe.input.iter().zip(machine.input.iter_mut())
                        {
                            hatch_input.buffer.take(recipe_input);
                        }

                        // put items in output hatches
                        for (recipe_output, hatch_output) in
                            recipe.output.iter().zip(machine.output.iter_mut())
                        {
                            hatch_output.buffer.accumulate(recipe_output);
                        }

                        // reset machine progress
                        machine.progress.take().unwrap();
                        true
                    }
                } else {
                    false
                }
            } else {
                true
            };

            if reset_progress {
                // reset consumption pole
                if let Some(consumer_pole_id) = machine.pole {
                    println!("reset power pole reset progress");
                    self.poles[*consumer_pole_id] = Pole::Consumer {
                        target_load: 0,
                        current_load: 0,
                    };
                }

                machine.status = MachineStatus::None;

                if let Some(recipe) = machine.recipe {
                    assert_eq!(recipe.input.len(), machine.input.len());
                    assert_eq!(recipe.output.len(), machine.output.len());

                    let inputs_match_recipe_input =
                        recipe.input.iter().zip(machine.input.iter()).all(
                            |(recipe_input_item, input_hatch)| {
                                input_hatch.buffer.id == recipe_input_item.id
                                    && input_hatch.buffer.count >= recipe_input_item.count
                            },
                        );
                    let outputs_match_recipe_output =
                        recipe.output.iter().zip(machine.output.iter()).all(
                            |(recipe_output_item, output_hatch)| {
                                if output_hatch.buffer.is_invalid() {
                                    return true;
                                }

                                // if the item is the same, must make sure that we have enough space in the hatch to place it 
                                let same_id = output_hatch.buffer.id == recipe_output_item.id;
                                let stack_size = REGISTRY[output_hatch.buffer.id as usize].stack_size;

                                // this CAN overflow if stack size is at MAX
                                // if we know it will overflow, then we cannot process the recipe
                                let opt_non_overflowing_enough_space_considering_stack_size = output_hatch.buffer.count.checked_add(recipe_output_item.count).map(|x| x <= stack_size);
                                same_id && opt_non_overflowing_enough_space_considering_stack_size.unwrap_or_default()
                            },
                        );

                    // if requirements are met, then we can begin machine progress
                    if inputs_match_recipe_input && outputs_match_recipe_output {
                        let _ = machine.progress.insert(Progress {
                            ticks_remaining: NonZeroU16::new(recipe.ticks)
                                .expect("recipe ticks must not be zero"),
                            slow_down_ticks_remaining: None,
                        });

                        // set the machine's consumer pole to enabled state
                        if let Some(consumer_pole_id) = machine.pole {
                            println!("set consumer power pole values");
                            self.poles[*consumer_pole_id] = Pole::Consumer {
                                target_load: recipe.load,
                                current_load: 0,
                            };
                            dbg!(&self.poles[*consumer_pole_id]);
                        }
                    } else {
                        if !inputs_match_recipe_input {
                            machine.status = MachineStatus::RecipeInputResourcesMismatchOrEmpty;
                        }

                        if !outputs_match_recipe_output {
                            machine.status = MachineStatus::RecipeOutputResourcesMismatchOrFull;
                        }
                    }
                }
            }
        }

        for belt in belts.iter_mut() {
            let Belt {
                belt_start:
                    HatchReference {
                        machine_index: start,
                        hatch_index: start_hatch_index,
                    },
                belt_end:
                    HatchReference {
                        machine_index: end,
                        hatch_index: end_hatch_index,
                    },
                ref mut buffer,
            } = *belt;

            // belt start
            let output_hatch = &machines[start].output[start_hatch_index];

            // belt end
            let input_hatch = &machines[end].input[end_hatch_index];

            // let belt_has_input = !output_hatch.buffer.is_invalid();
            let belt_free_output = input_hatch.buffer.is_invalid()
                || input_hatch.buffer.id
                    == buffer
                        .last()
                        .map(|x| x.id)
                        .unwrap_or(output_hatch.buffer.id);

            // order of operations:
            // transfer last element in belt buffer to input hatch
            // roll elements in belt buffer one to the right
            // transfer output hatch item to first element in belt buffer
            if belt_free_output {
                // element at index buffer.len()-1 is belt output
                let input_hatch = &mut machines[end].input[end_hatch_index];
                input_hatch.buffer = *buffer.last().unwrap();
                *buffer.last_mut().unwrap() = Item::invalid();

                println!("belt moving");
                buffer.rotate_right(1);

                // element at index 0 is the belt input
                // belt takes input from hatch...
                let output_hatch = &mut machines[start].output[start_hatch_index];
                buffer[0] = output_hatch.buffer;
                output_hatch.buffer = Item::invalid();
            }
        }
    }
}

fn main() {
    /*
    let mut machines = (0..1)
        .into_iter()
        .map(|_| {
            let mut crusher = Machine {
                input: vec![Hatch::empty()],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                progress: None,
                pole: None,
            };

            crusher.input[0] = Hatch {
                buffer: Item {
                    id: RAW_IRON_1,
                    count: 25,
                },
            };

            crusher
        })
        .collect::<Vec<_>>();

    machines.push(Machine {
        input: vec![Hatch::empty()],
        output: vec![Hatch::empty()],
        recipe: Some(&CRUSH_IRON_MORE_RECIPE),
        progress: None,
        pole: None,
    });

    let belts = vec![Belt {
        belt_start: HatchReference {
            machine_index: machines.len() - 2,
            hatch_index: 0,
        },
        belt_end: HatchReference {
            machine_index: machines.len() - 1,
            hatch_index: 0,
        },
        buffer: vec![Item::invalid(); 64],
    }];

    let poles = vec![
        Pole::Generator {
            max_load: 10,
            current_load: 0,
        },
        Pole::Generator {
            max_load: 1,
            current_load: 0,
        },
        Pole::Consumer {
            target_load: 10,
            current_load: 0,
        },
    ];
    let wires = vec![wire(0, 2), wire(1, 2)];

    let mut game = Game {
        machines,
        belts,
        poles,
        wires,
    };

    for _ in 0..10 {
        game.tick();
    }
    */
}

mod tests {
    use super::*;


    #[allow(dead_code)]
    fn wire(a: usize, b: usize) -> Wire {
        Wire {
            a: PoleId::from(a),
            b: PoleId::from(b),
            flow: 0,
        }
    }

    #[allow(dead_code)]
    fn placeholder_machine_with_output_hatch(tmp: Item) -> Machine {
        Machine {
            output: vec![Hatch { buffer: tmp }],
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    fn placeholder_machine_with_input_hatch(tmp: Item) -> Machine {
        Machine {
            input: vec![Hatch { buffer: tmp }],
            ..Default::default()
        }
    }


    #[test]
    fn empty() {
        Game::default().tick();
    }

    #[test]
    fn simple_power() {
        let mut game = Game {
            poles: vec![
                Pole::Generator {
                    max_load: 10,
                    current_load: 0,
                },
                Pole::Consumer {
                    target_load: 10,
                    current_load: 0,
                },
            ],
            wires: vec![wire(0, 1)],
            ..Default::default()
        };

        game.tick();
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(matches!(
            game.poles[1],
            Pole::Consumer {
                current_load: 10,
                ..
            }
        ));
        assert!(game.wires[0].flow == 10);
    }

    #[test]
    fn simple_power_inv() {
        let mut game = Game {
            poles: vec![
                Pole::Generator {
                    max_load: 10,
                    current_load: 0,
                },
                Pole::Consumer {
                    target_load: 10,
                    current_load: 0,
                },
            ],
            wires: vec![wire(1, 0)],
            ..Default::default()
        };

        game.tick();
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(matches!(
            game.poles[1],
            Pole::Consumer {
                current_load: 10,
                ..
            }
        ));
        assert!(game.wires[0].flow == -10);
    }

    #[test]
    fn simple_power_inv_2() {
        let mut game = Game {
            poles: vec![
                Pole::Consumer {
                    target_load: 10,
                    current_load: 0,
                },
                Pole::Generator {
                    max_load: 10,
                    current_load: 0,
                },
            ],
            wires: vec![wire(1, 0)],
            ..Default::default()
        };

        game.tick();
        assert!(matches!(
            game.poles[1],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(matches!(
            game.poles[0],
            Pole::Consumer {
                current_load: 10,
                ..
            }
        ));
        assert!(game.wires[0].flow == 10);
    }

    #[test]
    fn simple_power_split_load_equally() {
        let mut game = Game {
            poles: vec![
                Pole::Generator {
                    max_load: 10,
                    current_load: 0,
                },
                Pole::Consumer {
                    target_load: 5,
                    current_load: 0,
                },
                Pole::Consumer {
                    target_load: 5,
                    current_load: 0,
                },
            ],
            wires: vec![wire(0, 1), wire(0, 2)],
            ..Default::default()
        };

        game.tick();
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(matches!(
            game.poles[1],
            Pole::Consumer {
                current_load: 5,
                ..
            }
        ));
        assert!(matches!(
            game.poles[2],
            Pole::Consumer {
                current_load: 5,
                ..
            }
        ));
        assert!(game.wires[0].flow == 5);
        assert!(game.wires[1].flow == 5);
    }

    #[test]
    fn simple_power_split_load_inequally() {
        let mut game = Game {
            poles: vec![
                Pole::Generator {
                    max_load: 10,
                    current_load: 0,
                },
                Pole::Consumer {
                    target_load: 7,
                    current_load: 0,
                },
                Pole::Consumer {
                    target_load: 3,
                    current_load: 0,
                },
            ],
            wires: vec![wire(0, 1), wire(0, 2)],
            ..Default::default()
        };

        game.tick();
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(matches!(
            game.poles[1],
            Pole::Consumer {
                current_load: 7,
                ..
            }
        ));
        assert!(matches!(
            game.poles[2],
            Pole::Consumer {
                current_load: 3,
                ..
            }
        ));
        assert!(game.wires[0].flow == 7);
        assert!(game.wires[1].flow == 3);
    }

    #[test]
    fn simple_power_chain() {
        let mut poles = vec![];
        poles.push(Pole::Generator {
            max_load: 10,
            current_load: 0,
        });
        poles.extend(std::iter::repeat_with(|| Pole::Other).take(32));
        poles.push(Pole::Consumer {
            target_load: 10,
            current_load: 0,
        });

        let wires = (0..33)
            .into_iter()
            .map(|i| wire(i, i + 1))
            .collect::<Vec<_>>();

        let mut game = Game {
            poles,
            wires,
            ..Default::default()
        };

        for wire in game.wires.iter() {
            assert_eq!(wire.flow, 0);
        }

        for pole in game.poles.iter() {
            match pole {
                Pole::Generator { current_load, .. } => assert_eq!(*current_load, 0),
                Pole::Consumer { current_load, .. } => assert_eq!(*current_load, 0),
                Pole::Other => {}
            };
        }

        game.tick();

        for wire in game.wires.iter() {
            assert_eq!(wire.flow, 10);
        }

        for pole in game.poles.iter() {
            match pole {
                Pole::Generator { current_load, .. } => assert_eq!(*current_load, 10),
                Pole::Consumer { current_load, .. } => assert_eq!(*current_load, 10),
                Pole::Other => {}
            };
        }
    }

    #[test]
    fn simple_machine_power() {
        let mut game = Game {
            poles: vec![
                Pole::Generator {
                    max_load: 10,
                    current_load: 0,
                },
                Pole::Consumer {
                    target_load: 0,
                    current_load: 0,
                },
            ],
            wires: vec![wire(0, 1)],
            machines: vec![Machine {
                input: vec![Hatch {
                    buffer: CRUSH_IRON_RECIPE.input[0],
                }],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                progress: None,
                pole: Some(PoleId::from(1)),
                ..Default::default()
            }],
            ..Default::default()
        };

        // no power yet
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(matches!(
            game.poles[1],
            Pole::Consumer {
                current_load: 0,
                ..
            }
        ));
        assert!(game.wires[0].flow == 0);
        assert!(game.machines[0].progress.is_none());

        game.tick();

        // first tick simply sets recipe and TARGET load of consumer
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(
            matches!(game.poles[1], Pole::Consumer { current_load: 0, target_load } if target_load == CRUSH_IRON_RECIPE.load)
        );
        assert!(game.wires[0].flow == 0);
        assert!(game.machines[0].progress.is_some());

        game.tick();

        dbg!(&game.poles);

        // second tick actually propagates power generation
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 10,
                ..
            }
        ));
        assert!(
            matches!(game.poles[1], Pole::Consumer { current_load, target_load } if target_load == CRUSH_IRON_RECIPE.load && current_load == CRUSH_IRON_RECIPE.load)
        );
        assert!(game.wires[0].flow == 10);
        assert!(matches!(game.machines[0].status, MachineStatus::None));
        assert!(matches!(game.machines[0].progress, Some(Progress { slow_down_ticks_remaining, .. }) if slow_down_ticks_remaining.is_none() ));
    }

    #[test]
    fn simple_machine_underpowered() {
        let mut game = Game {
            poles: vec![
                Pole::Generator {
                    max_load: 2,
                    current_load: 0,
                },
                Pole::Consumer {
                    target_load: 0,
                    current_load: 0,
                },
            ],
            wires: vec![wire(0, 1)],
            machines: vec![Machine {
                input: vec![Hatch {
                    buffer: CRUSH_IRON_RECIPE.input[0],
                }],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                progress: None,
                pole: Some(PoleId::from(1)),
                ..Default::default()
            }],
            ..Default::default()
        };

        // no power yet
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(matches!(
            game.poles[1],
            Pole::Consumer {
                current_load: 0,
                ..
            }
        ));
        assert!(game.wires[0].flow == 0);
        assert!(game.machines[0].progress.is_none());

        game.tick();

        // first tick simply sets recipe and TARGET load of consumer
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(
            matches!(game.poles[1], Pole::Consumer { current_load: 0, target_load } if target_load == CRUSH_IRON_RECIPE.load)
        );
        assert!(game.wires[0].flow == 0);
        assert!(game.machines[0].progress.is_some());

        game.tick();

        dbg!(&game.poles);

        // second tick actually propagates power generation
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 2,
                ..
            }
        ));
        assert!(
            matches!(game.poles[1], Pole::Consumer { current_load, target_load } if target_load == CRUSH_IRON_RECIPE.load && current_load == 2)
        );
        assert!(game.wires[0].flow == 2);
        assert!(matches!(game.machines[0].status, MachineStatus::Underpowered));
        assert!(matches!(game.machines[0].progress, Some(Progress { slow_down_ticks_remaining, .. }) if slow_down_ticks_remaining.is_some() ));        
    }

    
    #[test]
    fn simple_machine_non_powered() {
        let mut game = Game {
            poles: vec![
                Pole::Generator {
                    max_load: 0,
                    current_load: 0,
                },
                Pole::Consumer {
                    target_load: 0,
                    current_load: 0,
                },
            ],
            wires: vec![wire(0, 1)],
            machines: vec![Machine {
                input: vec![Hatch {
                    buffer: CRUSH_IRON_RECIPE.input[0],
                }],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                progress: None,
                pole: Some(PoleId::from(1)),
                ..Default::default()
            }],
            ..Default::default()
        };

        // no power yet
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(matches!(
            game.poles[1],
            Pole::Consumer {
                current_load: 0,
                ..
            }
        ));
        assert!(game.wires[0].flow == 0);
        assert!(game.machines[0].progress.is_none());

        game.tick();

        // first tick simply sets recipe and TARGET load of consumer
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(
            matches!(game.poles[1], Pole::Consumer { current_load: 0, target_load } if target_load == CRUSH_IRON_RECIPE.load)
        );
        assert!(game.wires[0].flow == 0);
        assert!(game.machines[0].progress.is_some());

        game.tick();

        dbg!(&game.poles);

        // second tick actually propagates power generation
        assert!(matches!(
            game.poles[0],
            Pole::Generator {
                current_load: 0,
                ..
            }
        ));
        assert!(
            matches!(game.poles[1], Pole::Consumer { current_load, target_load } if target_load == CRUSH_IRON_RECIPE.load && current_load == 0)
        );
        assert!(game.wires[0].flow == 0);
        assert!(matches!(game.machines[0].status, MachineStatus::NonPowered));
        assert!(matches!(game.machines[0].progress, Some(Progress { slow_down_ticks_remaining, .. }) if slow_down_ticks_remaining == NonZeroU16::new(u16::MAX) ));        
    }

    #[test]
    fn simple_machine_missing_input_ingredients() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch::empty()],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                ..Default::default()
            }],
            ..Default::default()
        };

        assert!(game.machines[0].progress.is_none());

        game.tick();

        // still none because the input hatch does not have the required input
        assert!(game.machines[0].progress.is_none());
    }

    #[test]
    fn correct_tick_amount() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch {
                    buffer: CRUSH_IRON_RECIPE.input[0],
                }],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                ..Default::default()
            }],
            ..Default::default()
        };

        assert!(game.machines[0].progress.is_none());

        // first tick will set the PROGRESS ticks to `CRUSH_IRON_RECIPE.ticks``
        game.tick();

        // elapsed `CRUSH_IRON_RECIPE.ticks`-1...
        for i in 0..(CRUSH_IRON_RECIPE.ticks - 1) {
            let expected_ticks_remaining = CRUSH_IRON_RECIPE.ticks - i;

            assert!(game.machines[0].progress.is_some());
            assert!(
                matches!(game.machines[0].progress, Some(Progress { ticks_remaining, .. }) if ticks_remaining == NonZeroU16::new(expected_ticks_remaining).unwrap())
            );

            game.tick();
        }

        game.tick();

        // should now be none because in TOTAL ever since the first tick, CRUSH_IRON_RECIPE.ticks have occurred
        assert!(game.machines[0].progress.is_none());
    }

    #[test]
    fn belt_simple_test() {
        let belts = vec![Belt {
            belt_start: HatchReference {
                machine_index: 0,
                hatch_index: 0,
            },
            belt_end: HatchReference {
                machine_index: 1,
                hatch_index: 0,
            },
            buffer: vec![Item::invalid(); 64],
        }];

        let mut game = Game {
            machines: vec![
                placeholder_machine_with_output_hatch(Item {
                    id: IRON_DUST,
                    count: 1,
                }),
                placeholder_machine_with_input_hatch(Item::invalid()),
            ],
            belts,
            poles: vec![],
            wires: vec![],
        };

        assert!(game.belts[0].buffer[0].is_invalid());

        for i in 0..65 {
            println!("starting tick {i}");
            game.tick();

            for buffer_element_index in 0..64 {
                dbg!(buffer_element_index);
                let item = game.belts[0].buffer[buffer_element_index];
                dbg!(item);

                if buffer_element_index == i {
                    assert_eq!(item.id, IRON_DUST);
                    assert_eq!(item.count, 1);
                } else {
                    assert!(item.is_invalid());
                }
            }
        }
    }

    #[test]
    fn craft_tick_order() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch::item(RAW_IRON_1, CRUSH_IRON_RECIPE.input[0].count)],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert!(game.machines[0].output[0].buffer.is_invalid());
        assert!(game.machines[0].progress.is_none());

        assert_eq!(game.machines[0].input[0].buffer, CRUSH_IRON_RECIPE.input[0]);

        // first tick to start the recipe process
        game.tick();

        let initial_ticks = NonZeroU16::new(CRUSH_IRON_RECIPE.ticks).unwrap();
        assert!(matches!(
            game.machines[0].progress,
            Some(Progress {
                ticks_remaining,
                ..
            }) if initial_ticks == ticks_remaining
        ));

        // tick through recipe ticks...
        for _ in 0..(initial_ticks.get()) {
            game.tick();
        }

        assert!(game.machines[0].input[0].buffer.is_invalid());
        assert!(game.machines[0].progress.is_none());

        assert_eq!(
            game.machines[0].output[0].buffer,
            CRUSH_IRON_RECIPE.output[0]
        );
    }

    #[test]
    fn craft_batch() {
        let batch_count = 10;

        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch::item(
                    RAW_IRON_1,
                    batch_count * CRUSH_IRON_RECIPE.input[0].count,
                )],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert!(game.machines[0].output[0].buffer.is_invalid());

        for _ in 0..(CRUSH_IRON_RECIPE.ticks as usize * (batch_count as usize + 1)) {
            game.tick();
        }

        assert_eq!(game.machines[0].output[0].buffer.id, CRUSHED_IRON);
        assert_eq!(
            game.machines[0].output[0].buffer.count,
            CRUSH_IRON_RECIPE.output[0].count * batch_count
        );
    }

    #[test]
    fn craft_missing_inputs() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch::empty()],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert!(game.machines[0].output[0].buffer.is_invalid());

        game.tick();

        assert!(game.machines[0].output[0].buffer.is_invalid());
        assert_eq!(game.machines[0].status, MachineStatus::RecipeInputResourcesMismatchOrEmpty);
    }

    
    #[test]
    fn craft_missing_inputs_partial() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch::item(RAW_IRON_1, 1)],
                output: vec![Hatch::empty()],
                recipe: Some(&CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert!(game.machines[0].output[0].buffer.is_invalid());

        game.tick();

        assert!(game.machines[0].output[0].buffer.is_invalid());
        assert_eq!(game.machines[0].status, MachineStatus::RecipeInputResourcesMismatchOrEmpty);
    }

    #[test]
    fn craft_full_stack_outputs() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch { buffer: CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE.input[0] }],
                output: vec![Hatch { buffer: Item::full_stack(CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE.output[0].id) }],
                recipe: Some(&CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert_eq!(game.machines[0].status, MachineStatus::None);
        assert!(game.machines[0].progress.is_none());

        game.tick();

        assert_eq!(game.machines[0].status, MachineStatus::RecipeOutputResourcesMismatchOrFull);
    }

    #[test]
    fn craft_full_mismatch_id_outputs() {
        let mut game = Game {
            machines: vec![Machine {
                input: vec![Hatch { buffer: CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE.input[0] }],
                output: vec![Hatch { buffer: Item::full_stack(IRON_DUST) }],
                recipe: Some(&CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE),
                ..Default::default()
            }],
            belts: vec![],
            poles: vec![],
            wires: vec![],
        };

        assert_eq!(game.machines[0].status, MachineStatus::None);
        assert!(game.machines[0].progress.is_none());

        game.tick();

        assert!(game.machines[0].progress.is_none());
        assert_eq!(game.machines[0].status, MachineStatus::RecipeOutputResourcesMismatchOrFull);
    }
}
