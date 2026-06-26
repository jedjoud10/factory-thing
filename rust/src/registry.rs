use std::{collections::HashMap, sync::OnceLock};

use crate::{Item, LoadUnit};

pub trait Registry: Default {
    fn registry_item(id: u8) -> &'static RegistryItem;


    fn stack_size(id: u8) -> u8 {
        Self::registry_item(id).stack_size
    }

    fn name(id: u8) -> &'static str {
        Self::registry_item(id).name
    }
}

#[derive(Default)]
pub struct DefaultRegistry;

impl DefaultRegistry {
    pub const RAW_IRON_1: u8 = 1;
    pub const CRUSHED_IRON: u8 = 2;
    pub const IRON_DUST: u8 = 3;
    pub const IRON_INGOT: u8 = 4;

    pub const ITEMS: &[RegistryItem] = &[
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

    pub const CRUSH_IRON_RECIPE: Recipe = Recipe {
        input: &[Item::one(Self::RAW_IRON_1)],
        output: &[Item::one(Self::CRUSHED_IRON)],
        ticks: 16,
        load: 10,
    };

    pub const CRUSH_IRON_MORE_RECIPE: Recipe = Recipe {
        input: &[Item::one(Self::CRUSHED_IRON)],
        output: &[Item::one(Self::IRON_DUST)],
        ticks: 16,
        load: 10,
    };

    pub const SMELT_IRON_RECIPE: Recipe = Recipe {
        input: &[Item::one(Self::IRON_DUST)],
        output: &[Item::one(Self::IRON_INGOT)],
        ticks: 4,
        load: 0,
    };
}

impl Registry for DefaultRegistry {
    fn registry_item(id: u8) -> &'static RegistryItem {
        &Self::ITEMS[id as usize]
    }
}


#[derive(Default)]
pub struct TestRegistry;

impl TestRegistry {
    pub const RAW_IRON_1: u8 = 1;
    pub const CRUSHED_IRON: u8 = 2;
    pub const IRON_DUST: u8 = 3;
    pub const IRON_INGOT: u8 = 4;

    pub const ITEMS: &[RegistryItem] = &[
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

    pub const CRUSH_IRON_RECIPE: Recipe = Recipe {
        input: &[Item::one(Self::RAW_IRON_1)],
        output: &[Item::one(Self::CRUSHED_IRON)],
        ticks: 16,
        load: 10,
    };

    pub const CRUSH_IRON_MORE_RECIPE: Recipe = Recipe {
        input: &[Item::one(Self::CRUSHED_IRON)],
        output: &[Item::one(Self::IRON_DUST)],
        ticks: 16,
        load: 10,
    };

    pub const SMELT_IRON_RECIPE: Recipe = Recipe {
        input: &[Item::one(Self::IRON_DUST)],
        output: &[Item::one(Self::IRON_INGOT)],
        ticks: 4,
        load: 0,
    };
}

impl Registry for TestRegistry {
    fn registry_item(id: u8) -> &'static RegistryItem {
        &Self::ITEMS[id as usize]
    }
}


#[derive(PartialEq, Eq, Debug)]
pub struct RegistryItem {
    pub name: &'static str,
    pub stack_size: u8,
}

#[derive(Debug)]
pub struct Recipe {
    pub input: &'static [Item],
    pub output: &'static [Item],
    pub ticks: u16,
    pub load: LoadUnit,
}
