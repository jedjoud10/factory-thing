use std::{
    collections::{HashMap, HashSet, VecDeque},
    num::NonZeroU16,
};

use slotmap::{SlotMap, new_key_type};




use crate::{items::*, registry::{self, Recipe}};
use crate::handle::*;

pub type LoadUnit = isize;
pub type HealthUnit = u8;

#[derive(Debug)]
pub struct Belt {
    pub belt_start: HatchKey,
    pub belt_end: HatchKey,
    pub buffer: Vec<Item>,
    pub last_transfer_tick: u64,
}

pub enum BeltSize {
    BufferLength(u32),
    WorldLength(f32),
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
    pub input: Vec<HatchKey>,
    pub output: Vec<HatchKey>,
    pub recipe: Option<&'static Recipe>,
    pub progress: Option<Progress>,
    pub status: MachineStatus,
    pub pole: Option<PoleKey>,
}

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
    pub a: PoleKey,
    pub b: PoleKey,
    pub flow: LoadUnit,
    pub max_flow: LoadUnit,
    pub damage: HealthUnit,
}

#[derive(Debug)]
pub struct Silo {
    pub input: HatchKey,
    pub output: HatchKey,
    pub stack: Vec<Item>,
}

pub struct Settings {
    pub wire_damage_per_tick: Option<u8>,
    pub belt_buffer_scaling_factor: f32,
    pub belt_ticks_between_transfers: u64,
    pub belt_transfer_size: u8,
    pub silo_transfer_size: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            wire_damage_per_tick: None,
            belt_buffer_scaling_factor: 2f32,
            belt_ticks_between_transfers: 16,
            belt_transfer_size: 1,
            silo_transfer_size: 1,
        }
    }
}

#[derive(Default)]
pub struct Game<R: registry::Registry> {
    pub hatches: SlotMap<HatchKey, Hatch>,
    pub machines: SlotMap<MachineKey, Machine>,
    pub belts: SlotMap<BeltKey, Belt>,
    pub poles: SlotMap<PoleKey, Pole>,
    pub wires: SlotMap<WireKey, Wire>,
    pub silos: SlotMap<SiloKey, Silo>,
    pub settings: Settings,
    pub tick: u64,
    pub registry: R
}

#[derive(PartialEq, Eq)]
enum ResetProgressReason {
    NoPower,
    FinishedRecipe,
    NoProgress,
}

impl<R: registry::Registry> Game<R> {
    pub fn tick(&mut self) {
        let Self {
            machines,
            belts,
            poles,
            wires,
            hatches,
            tick,
            silos,
            ..
        } = self;

        // before we reset wire flow, check for max flow and do damage tick
        if let Some(wire_damage_per_tick) = self.settings.wire_damage_per_tick {
            for wire in wires.values_mut() {
                if wire.flow.abs() > wire.max_flow {
                    wire.damage = wire.damage.saturating_add(wire_damage_per_tick);
                }
            }

            wires.retain(|_, w| w.damage < u8::MAX);
        }


        // create adjacency map that stores neighbouring poles
        let mut lookup = HashMap::<PoleKey, Vec<(PoleKey, WireKey)>>::new();
        for (wire_key, Wire { a, b, .. }) in wires.iter() {
            lookup
                .entry(*a)
                .or_default()
                .push((*b, wire_key));
            lookup
                .entry(*b)
                .or_default()
                .push((*a, wire_key));
        }

        // reset load of poles
        for pole in poles.values_mut() {
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
        for wire in wires.values_mut() {
            wire.flow = 0;
        }

        // get all pole consumers
        let consumers = poles
            .iter()
            .filter_map(|(index, pole)| {
                if let Pole::Consumer { target_load, .. } = pole {
                    Some((index, *target_load))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        // for each consumer, it will run a BFS starting at the consumer pole and grow until it gets enough power 
        for (consumer_index, consumer_target_load) in consumers {
            assert!(consumer_target_load >= 0);
            let mut consumer_current_load = 0;

            let mut backtracking = HashMap::<PoleKey, (PoleKey, WireKey)>::new();
            let mut generators_used = HashSet::<(PoleKey, LoadUnit)>::new();
            let mut visited = HashSet::<PoleKey>::new();

            let mut queue = VecDeque::<PoleKey>::new();

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

                    let neighbour = &mut poles[*neighbour_index];

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

            match &mut poles[consumer_index] {
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

                        let wire = &mut wires[wire_index];
                        if wire.a == pole && wire.b == new_pole_id {
                            // `a` is closer to gen
                            // `b` is closer to con
                            wires[wire_index].flow += load; // flow from `a` to `b` is positive
                        } else if wire.b == pole && wire.a == new_pole_id {
                            // `a` is closer to con
                            // `b` is closer to gen
                            wires[wire_index].flow -= load; // flow from `a` to `b` is negative
                        } else {
                            unreachable!();
                        }
                    }
                }
            }
        }

        for machine in machines.values_mut() {
            let satisfied_and_target_load = machine.pole.map(|pole_id| match self.poles[pole_id] {
                Pole::Consumer {
                    target_load,
                    current_load,
                } => (current_load, target_load),
                _ => unreachable!(),
            });

            let opt_reset_progress = Self::do_progress(hatches, machine, satisfied_and_target_load);

            if let Some(reset_progress_reason) = opt_reset_progress {
                // reset consumption pole
                if let Some(consumer_pole_id) = machine.pole {
                    self.poles[consumer_pole_id] = Pole::Consumer {
                        target_load: 0,
                        current_load: 0,
                    };
                }

                // we must make sure to keep status otherwise it will get reset to "None" even though the reason was "NoPower"
                if reset_progress_reason == ResetProgressReason::NoPower {
                    machine.status = MachineStatus::NonPowered;
                } else {
                    machine.status = MachineStatus::None;
                }

                if let Some(recipe) = machine.recipe {
                    assert_eq!(recipe.input.len(), machine.input.len());
                    assert_eq!(recipe.output.len(), machine.output.len());

                    let inputs_match_recipe_input =
                        recipe.input.iter().zip(machine.input.iter()).all(
                            |(recipe_input_item, input_hatch)| {
                                let buffer = hatches[*input_hatch].buffer;
                                buffer.id == recipe_input_item.id
                                    && buffer.count >= recipe_input_item.count
                            },
                        );
                    let outputs_match_recipe_output =
                        recipe.output.iter().zip(machine.output.iter()).all(
                            |(recipe_output_item, output_hatch)| {
                                let buffer = hatches[*output_hatch].buffer;
                                if buffer.is_invalid() {
                                    return true;
                                }

                                // if the item is the same, must make sure that we have enough space in the hatch to place it 
                                let same_id = buffer.id == recipe_output_item.id;
                                let stack_size = R::stack_size(buffer.id);

                                // this CAN overflow if stack size is at MAX
                                // if we know it will overflow, then we cannot process the recipe
                                let opt_non_overflowing_enough_space_considering_stack_size = buffer.count.checked_add(recipe_output_item.count).map(|x| x <= stack_size);
                                same_id && opt_non_overflowing_enough_space_considering_stack_size.unwrap_or_default()
                            },
                        );

                    // if requirements are met, then we can begin machine progress
                    if inputs_match_recipe_input && outputs_match_recipe_output {
                        // resume the progress previously (can only happen if we lose power in the middle of a recipe)
                        if let Some(previous_progress) = machine.progress.take() {
                            // godot::global::godot_print!("progress already existed");
                            machine.progress = Some(previous_progress);
                        } else {
                            // godot::global::godot_print!("new progress");
                            let _ = machine.progress.insert(Progress {
                                ticks_remaining: NonZeroU16::new(recipe.ticks)
                                    .expect("recipe ticks must not be zero"),
                                slow_down_ticks_remaining: None,
                            });
                        }

                        // set the machine's consumer pole to enabled state
                        if let Some(consumer_pole_id) = machine.pole {
                            self.poles[consumer_pole_id] = Pole::Consumer {
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
            } else {
                //machine.status = MachineStatus::None
            }
        }

        for silo in silos.values_mut() {
            let input = &mut hatches[silo.input].buffer;
            let mut taken = Item::invalid();
            Item::transfer_limited::<R>(input, &mut taken, self.settings.silo_transfer_size);

            if !taken.is_invalid() {
                silo.stack.push(taken);
            }

            if let Some(last) = silo.stack.last_mut() {
                let predicate = hatches[silo.output].buffer.can_accumulate_from::<R>(last);

                if predicate {
                    let output = &mut hatches[silo.output].buffer;
                    Item::transfer_limited::<R>(last, output, self.settings.silo_transfer_size);

                    if last.is_invalid() {
                        silo.stack.pop().unwrap();
                    }
                }
            }
        }

        for belt in belts.values_mut() {
            let Belt {
                belt_start,
                belt_end,
                ref mut buffer,
                last_transfer_tick: ref mut last_transfer_ticks,
            } = *belt;

            // don't do anything if belt points to invalid hatches
            if !hatches.contains_key(belt_start) || !hatches.contains_key(belt_end) {
                continue;
            }

            // impossible that it overflows because self.tick > last_transfer_ticks, always
            if (*tick - *last_transfer_ticks) >= self.settings.belt_ticks_between_transfers {
                // godot::global::godot_print!("belt tick");
                
                let input_hatch = &hatches[belt_end];
                let predicate = input_hatch.buffer.is_invalid() || (input_hatch.buffer.can_accumulate_from::<R>(buffer.last().unwrap()) || buffer.last().unwrap().is_invalid());

                // order of operations:
                // transfer last element in belt buffer to input hatch
                // roll elements in belt buffer one to the right
                // transfer output hatch item to first element in belt buffer
                if predicate {
                    *last_transfer_ticks = *tick;

                    // godot::global::godot_print!("belt shifting");

                    // element at index buffer.len()-1 is belt output
                    let input_hatch = &mut hatches[belt_end];
                    Item::transfer_limited::<R>(buffer.last_mut().unwrap(), &mut input_hatch.buffer, self.settings.belt_transfer_size);

                    // I'm shifting... I'm shifting....
                    buffer.rotate_right(1);

                    // first element must be reset to invalid now...
                    buffer[0].invalidate();

                    // element at index 0 is the belt input
                    // belt takes input from hatch...
                    let output_hatch = &mut hatches[belt_start];
                    Item::transfer_limited::<R>(&mut output_hatch.buffer, &mut buffer[0], self.settings.belt_transfer_size);
                }
            }
        }

        *tick += 1;
    }

    fn do_progress(hatches: &mut SlotMap<HatchKey, Hatch>, machine: &mut Machine, satisfied_and_target_load: Option<(LoadUnit, LoadUnit)>) -> Option<ResetProgressReason> {
        // godot::global::godot_print!("{:?}", satisfied_and_target_load);
        let reset_progress = if let Some(progress) = machine.progress.as_mut() {
            if let Some((satisfied, target)) = satisfied_and_target_load {
                if satisfied == 0 {
                    // "infinite" slow down ticks for machines that are not powered at all
                    progress.slow_down_ticks_remaining = NonZeroU16::new(u16::MAX);
                    machine.status = MachineStatus::NonPowered;
                } else if satisfied < target {
                    // godot::global::godot_print!("underpowered");
                    machine.status = MachineStatus::Underpowered;
    
                    // can only update once this has been reset (in case fluctuating power) OR if it was previously set to specific value
                    if progress.slow_down_ticks_remaining.is_none() || progress.slow_down_ticks_remaining == Some(NonZeroU16::new(u16::MAX).unwrap())  {
                        // calculate efficiency percentage
                        let percent = satisfied as f32 / target as f32;
    
                        // 100% result in 1 ticks (which will get reset immediately after it gets set)
                        // 50% result in 2 ticks
                        // 25% result in 4 ticks
                        // TODO: replace this, as we cannot represent inefficiencies >50% but <100% in slow down ticks
                        let inv = (1.0f32 / percent) as u16;
                        progress.slow_down_ticks_remaining = NonZeroU16::new(inv);
                    }
                } else {
                    // no slow down for fully powered shii
                    progress.slow_down_ticks_remaining = None;
                }
            } else {
                // no slow down ticks for machines without poles
                progress.slow_down_ticks_remaining = None;
            }
    
            // prioritize slow down ticks first
            let progress_normally = if let Some(slow_down_ticks) = progress.slow_down_ticks_remaining {
                // special case: non powered machines 
                if slow_down_ticks.get() == u16::MAX {
                    // we must RESET progress
                    return Some(ResetProgressReason::NoPower);
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
                    None
                } else {
                    // machine finished the recipe (remaining ticks is zero, but no need to update it, as we invalidate `progress` anyways)
                    // `unwrap` here is safe because the `recipe: Option<&'static Recipe>` should not be set to `None` when a machine is progressing
                    let recipe = &machine.recipe.unwrap();
    
                    // take items from input hatches
                    for (recipe_input, hatch_input) in
                        recipe.input.iter().zip(machine.input.iter())
                    {
                        let buffer = &mut hatches[*hatch_input].buffer;
                        buffer.take(recipe_input);
                    }
    
                    // put items in output hatches
                    for (recipe_output, hatch_output) in
                        recipe.output.iter().zip(machine.output.iter())
                    {
                        let buffer = &mut hatches[*hatch_output].buffer;
                        buffer.accumulate::<R>(recipe_output);
                    }
    
                    // reset machine progress
                    machine.progress.take().unwrap();
                    Some(ResetProgressReason::FinishedRecipe)
                }
            } else {
                None
            }
        } else {
            Some(ResetProgressReason::NoProgress)
        };
        
        reset_progress
    }
}

impl<R: registry::Registry> Game<R> {
    pub fn add_machine_with_pole(&mut self, recipe: &'static Recipe, pole_key: PoleKey) -> MachineKey {
        self.poles[pole_key] = Pole::Consumer { target_load: 0, current_load: 0 };

        // TODO: scale number of hatches with required recipe I/O
        let input = self.hatches.insert(Hatch::empty());
        let output = self.hatches.insert(Hatch::empty());
        
        let machine = Machine {
            input: vec![input],
            output: vec![output],
            recipe: Some(&recipe),
            progress: None,
            pole: Some(pole_key),
            ..Default::default()
        };

        self.machines.insert(machine)
    }

    pub fn add_machine(&mut self, recipe: &'static Recipe) -> (MachineKey, PoleKey) {
        let pole_id = self.poles.insert(Pole::Consumer { target_load: 0, current_load: 0 });

        // TODO: scale number of hatches with required recipe I/O
        let input = self.hatches.insert(Hatch::empty());
        let output = self.hatches.insert(Hatch::empty());

        let machine = Machine {
            input: vec![input],
            output: vec![output],
            recipe: Some(&recipe),
            progress: None,
            pole: Some(pole_id),
            ..Default::default()
        };

        let machine_id = self.machines.insert(machine);
        (machine_id, pole_id)
    }

    pub fn remove_machine(&mut self, key: MachineKey) {
        let m = self.machines.remove(key).unwrap();
        for hatch in m.input.iter().chain(m.output.iter()) {
            self.hatches.remove(*hatch).unwrap();
        }
    }

    pub fn add_silo(&mut self) -> SiloKey {
        let input = self.hatches.insert(Hatch::empty());
        let output = self.hatches.insert(Hatch::empty());

        self.silos.insert(Silo {
            input,
            output,
            stack: Vec::default(),
        })
    }

    pub fn remove_silo(&mut self, key: SiloKey) {
        let m = self.silos.remove(key).unwrap();
        self.hatches.remove(m.input);
        self.hatches.remove(m.output);
    }

    // not really adding...
    pub fn add_generator_with_pole(&mut self, max_load: LoadUnit, pole_key: PoleKey) {
        self.poles[pole_key] = Pole::Generator { max_load, current_load: 0 }
    }
    
    pub fn add_infinite_generator(&mut self) -> PoleKey {
        self.add_generator(LoadUnit::MAX)
    }

    pub fn add_generator(&mut self, max_load: LoadUnit) -> PoleKey {
        self.poles.insert(Pole::Generator { max_load, current_load: 0 })
    }

    pub fn add_consumer(&mut self, target_load: LoadUnit) -> PoleKey {
        self.poles.insert(Pole::Consumer { target_load, current_load: 0 })
    }

    pub fn add_pole(&mut self) -> PoleKey {
        self.poles.insert(Pole::Other)
    }

    pub fn remove_pole(&mut self, key: PoleKey) {
        self.poles.remove(key);

        // remove associated wires
        self.wires.retain(|_, w| !(w.a == key || w.b == key));
    } 
    
    pub fn add_wire_with_max_flow(&mut self, a: PoleKey, b: PoleKey, max_flow: LoadUnit) -> WireKey {
        if self.wires.values().any(|wire| (wire.a == a && wire.b == b) || (wire.b == a && wire.a == b)) {
            panic!("cannot add duplicate wire");
        }

        self.wires.insert(Wire {
            a, b,
            flow: 0,
            damage: 0,
            max_flow,
        })
    }

    pub fn add_wire(&mut self, a: PoleKey, b: PoleKey) -> WireKey {
        assert!(a != b);

        self.add_wire_with_max_flow(a, b, LoadUnit::MAX)
    }

    pub fn remove_wire(&mut self, wire: WireKey) {
        self.wires.remove(wire);
    }

    pub fn add_wire_chain(&mut self, poles: &[PoleKey]) -> Vec<WireKey> {
        poles.windows(2).into_iter().map(|window| {
            self.add_wire(window[0], window[1])
        }).collect::<Vec<WireKey>>()
    }

    pub fn add_belt(&mut self, output_hatch: HatchKey, input_hatch: HatchKey) -> BeltKey {
        assert!(output_hatch != input_hatch);

        self.belts.insert(Belt {
            belt_start: output_hatch,
            belt_end: input_hatch,
            buffer: vec![Item::invalid(); 8],
            last_transfer_tick: 0,
        })
    }

    pub fn add_belt_2(&mut self, output_hatch: HatchKey, input_hatch: HatchKey, length: BeltSize) -> BeltKey {
        assert!(output_hatch != input_hatch);

        let buffer_length = match length {
            BeltSize::BufferLength(x) => x as f32,
            BeltSize::WorldLength(x) => x * self.settings.belt_buffer_scaling_factor,
        }.ceil().max(2f32) as usize;

        self.belts.insert(Belt {
            belt_start: output_hatch,
            belt_end: input_hatch,
            buffer: vec![Item::invalid(); buffer_length],
            last_transfer_tick: 0
        })
    }

    pub fn remove_belt(&mut self, belt: BeltKey) {
        self.belts.remove(belt);
    }
    
    
    pub fn get_input_hatch_mut(&mut self, machine_id: MachineKey, hatch_index: usize) -> &mut Item {
        &mut self.hatches[self.machines[machine_id].input[hatch_index]].buffer
    }

    pub fn get_output_hatch_mut(&mut self, machine_id: MachineKey, hatch_index: usize) -> &mut Item {
        &mut self.hatches[self.machines[machine_id].output[hatch_index]].buffer
    }
}