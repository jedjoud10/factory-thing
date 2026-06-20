use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZeroU16,
};

use crate::items::*;
use crate::handle::*;

pub type MachineId = usize;
pub type LoadUnit = isize;
pub type HealthUnit = u8;

#[derive(Debug)]
pub struct HatchReference {
    pub machine_index: MachineId,

    // we inherently know if this is an Input or Output hatch since the Belt stores the belt_start and belt_end information
    // ex: belt_end could only refer to input hatches since that's the only hatch direction they can connect to
    pub hatch_index: usize,
}

#[derive(Debug)]
pub struct Belt {
    pub belt_start: HatchReference,
    pub belt_end: HatchReference,
    pub buffer: Vec<Item>,
}

#[derive(Debug)]
pub struct Hatch {
    pub buffer: Item,
}

impl Hatch {
    pub const fn item(id: u8, count: u8) -> Self {
        Self {
            buffer: Item { id, count },
        }
    }

    pub const fn empty() -> Self {
        Self {
            buffer: Item::invalid(),
        }
    }
}

#[derive(Debug)]
pub struct Progress {
    pub ticks_remaining: NonZeroU16,
    
    // used if the machine is being underpowered / inefficient
    pub slow_down_ticks_remaining: Option<NonZeroU16>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub enum MachineStatus {
    RecipeInputResourcesMismatchOrEmpty,
    RecipeOutputResourcesMismatchOrFull,
    
    Underpowered,
    NonPowered,

    #[default]
    None,
}

#[derive(Default, Debug)]
pub struct Machine {
    pub input: Vec<Hatch>,
    pub output: Vec<Hatch>,
    pub recipe: Option<&'static Recipe>,
    pub progress: Option<Progress>,
    pub status: MachineStatus,
    pub pole: Option<PoleId>,
}

pub const RAW_IRON_1: u8 = 1;

pub const CRUSHED_IRON: u8 = 2;

pub const IRON_DUST: u8 = 3;

pub const IRON_INGOT: u8 = 4;

pub const REGISTRY: &'static [RegistryItem] = &[
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
pub struct Recipe {
    pub input: &'static [Item],
    pub output: &'static [Item],
    pub ticks: u16,
    pub load: LoadUnit,
}

pub const CRUSH_IRON_RECIPE: Recipe = Recipe {
    input: &[Item::one(RAW_IRON_1)],
    output: &[Item::one(CRUSHED_IRON)],
    ticks: 16,
    load: 10,
};

pub const CRUSH_IRON_ALTERNATIVE_BATCH_RECIPE: Recipe = Recipe {
    input: &[Item::new(RAW_IRON_1, 4)],
    output: &[Item::new(CRUSHED_IRON, 4)],
    ticks: 16,
    load: 10,
};

pub const CRUSH_IRON_MORE_RECIPE: Recipe = Recipe {
    input: &[Item::one(CRUSHED_IRON)],
    output: &[Item::one(IRON_DUST)],
    ticks: 16,
    load: 10,
};

pub const SMELT_IRON_RECIPE: Recipe = Recipe {
    input: &[Item::one(IRON_DUST)],
    output: &[Item::one(IRON_INGOT)],
    ticks: 4,
    load: 0,
};

#[derive(Debug)]
pub enum Pole {
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
pub struct Wire {
    pub a: PoleId,
    pub b: PoleId,
    pub flow: LoadUnit,
    pub max_flow: LoadUnit,
    pub damage: HealthUnit,
}

#[derive(Default)]
pub struct Game {
    pub machines: Vec<Machine>,
    pub belts: Vec<Belt>,
    pub poles: Vec<Pole>,
    pub wires: Vec<Wire>,
}

impl Game {
    pub fn tick(&mut self) {
        let machines = &mut self.machines;
        let belts = &mut self.belts;
        let poles = &mut self.poles;
        let wires = &mut self.wires;

        // before we reset wire flow, check for max flow and do damage tick
        for wire in wires.iter_mut() {
            if wire.flow.abs() > wire.max_flow {
                wire.damage = wire.damage.saturating_add(1);
            }
        }

        wires.retain(|w| w.damage < u8::MAX);

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

        // for each consumer, it will run a BFS starting at the consumer pole and grow until it gets enough power 
        for (consumer_index, consumer_target_load) in consumers {
            assert!(consumer_target_load >= 0);
            let mut consumer_current_load = 0;

            let mut backtracking = HashMap::<PoleId, (PoleId, WireId)>::new();
            let mut generators_used = HashSet::<(PoleId, LoadUnit)>::new();
            let mut visited = HashSet::<PoleId>::new();

            let mut queue = VecDeque::<PoleId>::new();

            // in case of pole but not connected to anything
            if lookup.contains_key(&consumer_index) {
                queue.push_back(consumer_index);
            }

            // simple BFS shortest-path search to find enough load to satisfy consumer
            'pathfind: while let Some(index) = queue.pop_front() {
                let neighbours = &lookup[&index];

                for (neighbour_index, wire_index) in neighbours {
                    let consumer_remaining_load_to_satisfy =
                        consumer_target_load - consumer_current_load;

                    assert!(consumer_remaining_load_to_satisfy >= 0);

                    // consumer has fully satisfied load, we can exit early
                    if consumer_remaining_load_to_satisfy == 0 {
                        break 'pathfind;
                    }

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
                            self.poles[*consumer_pole_id] = Pole::Consumer {
                                target_load: recipe.load,
                                current_load: 0,
                            };
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

                buffer.rotate_right(1);

                // element at index 0 is the belt input
                // belt takes input from hatch...
                let output_hatch = &mut machines[start].output[start_hatch_index];
                buffer[0] = output_hatch.buffer;
                output_hatch.buffer = Item::invalid();
            }
        }
    }
    
    pub fn add_machine(&mut self, recipe: &'static Recipe) -> (MachineId, PoleId) {
        let pole_id = self.poles.len();
        self.poles.push(Pole::Consumer { target_load: 0, current_load: 0 });

        let machine = Machine {
            input: vec![Hatch::empty()],
            output: vec![Hatch::empty()],
            recipe: Some(&recipe),
            progress: None,
            pole: Some(PoleId::from(pole_id)),
            ..Default::default()
        };

        let machine_id = self.machines.len();
        self.machines.push(machine);
        (machine_id, PoleId::from(pole_id))
    }
    
    pub fn add_infinite_generator(&mut self) -> PoleId {
        self.add_generator(LoadUnit::MAX)
    }

    
    pub fn add_generator(&mut self, max_load: LoadUnit) -> PoleId {
        let pole_id = self.poles.len();
        self.poles.push(Pole::Generator { max_load, current_load: 0 });
        PoleId::from(pole_id)
    }

    pub fn add_consumer(&mut self, target_load: LoadUnit) -> PoleId {
        let pole_id = self.poles.len();
        self.poles.push(Pole::Consumer { target_load, current_load: 0 });
        PoleId::from(pole_id)
    }

    pub fn add_pole(&mut self) -> PoleId {
        let pole_id = self.poles.len();
        self.poles.push(Pole::Other);
        PoleId::from(pole_id)
    }
    
    pub fn add_wire_with_max_flow(&mut self, a: PoleId, b: PoleId, max_flow: LoadUnit) {
        if self.wires.iter().any(|wire| (wire.a == a && wire.b == b) || (wire.b == a && wire.a == b)) {
            panic!("cannot add duplicate wire");
        }

        self.wires.push(Wire {
            a, b,
            flow: 0,
            damage: 0,
            max_flow,
        });
    }

    pub fn add_wire(&mut self, a: PoleId, b: PoleId) {
        self.add_wire_with_max_flow(a, b, LoadUnit::MAX);
    }

    pub fn add_wire_chain(&mut self, poles: &[PoleId]) {
        for window in poles.windows(2) {
            self.add_wire(window[0], window[1]);
        }
    }

    pub fn add_belt(&mut self, output_hatch: HatchReference, input_hatch: HatchReference) {
        self.belts.push(Belt {
            belt_start: output_hatch,
            belt_end: input_hatch,
            buffer: vec![Item::invalid(); 8],
        })
    }
    
    pub fn get_input_hatch_mut(&mut self, machine_id: usize, hatch_index: usize) -> &mut Item {
        &mut self.machines[machine_id].input[hatch_index].buffer
    }
}

fn main() {
    let mut game = Game::default();

    let pole_id_generator = game.add_infinite_generator();
    let other_pole = game.add_pole();
    game.add_wire(pole_id_generator, other_pole);

    let (machine_index, pole_id_machine) = game.add_machine(&CRUSH_IRON_RECIPE);
    game.get_input_hatch_mut(machine_index, 0).accumulate(&Item::new(RAW_IRON_1, 16));
    game.add_wire(other_pole, pole_id_machine);

    let (machine_index_2, pole_id_machine_2) = game.add_machine(&CRUSH_IRON_MORE_RECIPE);
    game.add_wire(other_pole, pole_id_machine_2);

    game.add_belt(HatchReference { machine_index, hatch_index: 0 }, HatchReference { machine_index: machine_index_2, hatch_index: 0 });    

    for _ in 0..3000 {
        game.tick();
    }

    dbg!(game.machines[machine_index_2].output[0].buffer);
}
