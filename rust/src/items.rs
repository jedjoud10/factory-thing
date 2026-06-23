use crate::REGISTRY;


#[derive(PartialEq, Eq, Debug)]
pub struct RegistryItem {
    pub name: &'static str,
    pub stack_size: u8,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Item {
    pub id: u8,
    pub count: u8,
}

impl Item {
    pub const fn one(id: u8) -> Self {
        Self { id, count: 1 }
    }

    pub const fn full_stack(id: u8) -> Self {
        Self { id, count: REGISTRY[id as usize].stack_size }
    }

    pub const fn new(id: u8, count: u8) -> Self {
        Self { id, count }
    }

    pub const fn invalid() -> Self {
        Self { id: 0, count: 0 }
    }

    pub const fn is_invalid(&self) -> bool {
        self.id == 0 && self.count == 0
    }

    pub const fn invalidate(&mut self) {
        *self = Self::invalid();
    }


    // `from` can be invalid
    // `to` can be invalid
    // this will do the transfer appropriately. modifies both
    pub fn transfer_limited(src: &mut Item, dst: &mut Item, max_transfer_count: u8) {
        if src.is_invalid() {
            return;
        }

        if dst.is_invalid() {
            // invalid target, move entire stack
            *dst = *src; 
            *src = Item::invalid();
        } else if dst.id == src.id {
            // same ID, do transfer but STACK LIMITED
            let stack_size = REGISTRY[src.id as usize].stack_size;

            // remaining slots in DST that can be filled
            let remaining_slots = stack_size - dst.count;

            // limited by src count as well
            let transferred_amount = remaining_slots.min(src.count);

            // limited by transfer count as well
            let transferred_amount = transferred_amount.min(max_transfer_count);
        
            assert!(transferred_amount <= stack_size);
            assert!(transferred_amount <= src.count);
            

            dst.count += transferred_amount;
            src.count -= transferred_amount;

            if src.count == 0 {
                src.invalidate();
            }
        } else {
            // different ID, don't accumulate
        }
    }

    pub const fn accumulate(&mut self, other: &Item) {
        assert!(self.is_invalid() || self.id == other.id);

        if !self.is_invalid() {
            let checked = self.count.checked_add(other.count);
            assert!(checked.is_some(), "accumulation count overflow integer type");
            let stack_size = REGISTRY[self.id as usize].stack_size;
            assert!(matches!(checked, Some(x) if x <= stack_size), "accumulation count overflow stack size");
        }

        self.id = other.id;
        self.count += other.count;
    }

    pub const fn can_accumulate_from(&self, other: &Item) -> bool {
        if other.is_invalid() {
            return false;
        }

        if self.is_invalid() {
            true
        } else if self.id == other.id && self.count + other.count <= REGISTRY[self.id as usize].stack_size {
            true
        } else {
            false
        }
    }

    pub fn display(&self) -> String {
        if self.is_invalid() {
            "Invalid".to_string()
        } else {
            format!("\"{}\" ({})", REGISTRY[self.id as usize].name, self.count)
        }
    }

    pub const fn take(&mut self, other: &Item) {
        assert!(!other.is_invalid());
        assert!(self.id == other.id);


        self.count = self.count - other.count;

        if self.count == 0 {
            self.invalidate();
        }
    }
}


mod item_tests {
    use super::*;
    use crate::stuff::*;


    #[test]
    fn single_item() {
        assert!(Item::invalid().is_invalid());

        let item = Item::one(RAW_IRON_1);
        assert!(!item.is_invalid());
        assert_eq!(item, Item { id: RAW_IRON_1, count: 1 });
    }

    #[test]
    fn invalidate_item() {
        let mut item = Item::one(RAW_IRON_1);
        assert!(!item.is_invalid());
        item.invalidate();
        assert!(item.is_invalid());
        
        let mut item = Item::one(RAW_IRON_1);
        assert!(!item.is_invalid());
        item.take(&Item::one(RAW_IRON_1));
        assert!(item.is_invalid());
    }

    #[test]
    fn take_item() {
        let mut item = Item::new(RAW_IRON_1, 10);
        assert_eq!(item.count, 10);
        item.take(&Item::one(RAW_IRON_1));
        assert_eq!(item.count, 9);
    }

    #[test]
    #[should_panic]
    fn take_item_too_much() {
        let mut item = Item::new(RAW_IRON_1, 1);
        assert_eq!(item.count, 1);

        // should cause a subtract with overflow; panic
        item.take(&Item::new(RAW_IRON_1, 10));
    }

    #[test]
    fn accumulate_item() {
        let mut item = Item::new(RAW_IRON_1, 10);
        assert_eq!(item.count, 10);
        item.accumulate(&Item::one(RAW_IRON_1));
        assert_eq!(item.count, 11);
    }

    #[test]
    #[should_panic]
    fn accumulate_item_stack_limit() {
        let mut item = Item::full_stack(RAW_IRON_1);
        assert_eq!(item.count, REGISTRY[RAW_IRON_1 as usize].stack_size);
        item.accumulate(&Item::one(RAW_IRON_1));
    }
}