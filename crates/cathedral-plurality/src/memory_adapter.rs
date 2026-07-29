use crate::PluralityClientTrait;

pub struct MemoryAdapter;

impl PluralityClientTrait for MemoryAdapter {
    fn new() -> Self {
        MemoryAdapter
    }
}
