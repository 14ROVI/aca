use std::collections::HashMap;

use bytes::{BufMut, BytesMut};

enum AllocationPolicy {
    DirectMapped,                                   // yk what this does
    FullyAssociative(HashMap<usize, usize>),        // parent line => cache line
    SetAssociative((usize, HashMap<usize, usize>)), // #sets,  set # => row in that set
}

enum ReplacementPolicy {
    LeastRecentlyUsed,
    LeastRecentlyReplaced,
    Random,
}

enum ConsistencyPolicy {
    WriteThrough,
    WriteBack,
}

struct CacheLineMeta {
    parent_index: usize,
    valid: bool,
    dirty: bool,
}

struct Memory {
    bytes: BytesMut,
    capcity: usize,
    speed: u32,
    cache_line_len: usize,
    cache_lines: usize,
    meta: HashMap<usize, CacheLineMeta>,
    upper_level: Option<Box<Memory>>,
    allocation_policy: AllocationPolicy,
    replacement_policy: ReplacementPolicy,
    consistency_policy: ConsistencyPolicy,
}
impl Memory {
    pub fn new(
        capcity: usize,
        speed: u32,
        cache_line_len: usize,
        upper_level: Option<Box<Memory>>,
        allocation_policy: AllocationPolicy,
        replacement_policy: ReplacementPolicy,
        consistency_policy: ConsistencyPolicy,
    ) -> Self {
        assert!(
            capcity % cache_line_len == 0,
            "Capacity has to be a multiple of cache line length!"
        );

        Self {
            bytes: BytesMut::new(),
            capcity,
            speed,
            cache_line_len,
            cache_lines: capcity / cache_line_len,
            meta: HashMap::new(),
            upper_level,
            allocation_policy: AllocationPolicy, // policy for interacting with level up
            replacement_policy: ReplacementPolicy, // policy for interacting with level up
            consistency_policy: ConsistencyPolicy, // policy for interacting with level up
        }
    }

    fn get_level_up_index(&self, this_index: usize) -> Option<usize> {
        let index_cache_line = this_index / self.cache_line_len;
        if let Some(meta) = self.meta.get(&index_cache_line) {
            let index_remainder = this_index % self.cache_line_len;
            return Some(meta.parent_index + index_remainder);
        } else {
            return None;
        }
    }

    fn get_level_down_index(&self, level_down: &Memory, this_index: usize) -> Option<usize> {
        match level_down.allocation_policy {
            AllocationPolicy::DirectMapped => {
                let cache_line = (this_index / level_down.cache_line_len) % level_down.cache_lines;
                let cache_remainder = this_index % level_down.cache_line_len;
                Some(cache_line + cache_remainder)
            }
            AllocationPolicy::FullyAssociative(map) => {
                let this_index_line = (this_index / self.cache_line_len);
                let cache_remainder = this_index % level_down.cache_line_len;
                let cache_line = map.get(&this_index_line);
                cache_line.map(|line| *line + cache_remainder)
            }
            AllocationPolicy::SetAssociative((num_sets, set_map)) => {
                // love child of direct and fully associative <3
                let this_index_line = this_index / self.cache_line_len;
                let set_index =
                    (this_index / level_down.cache_line_len) % (level_down.cache_lines / num_sets);
                let cache_remainder = this_index % level_down.cache_line_len;

                set_map
                    .get(&this_index)
                    .map(|set_line_offset| set_index + *set_line_offset + cache_remainder)
            }
        }
    }

    /// returns the time needed to get these bytes, and the bytes
    /// also fills up cache if missed based on policies
    pub fn get_bytes(&self, mut index: usize, count: usize) -> Option<(u32, &[u8])> {
        // how this function works:
        // we are the lowest level of the stack - we get the index from our parents translating it down to us
        // then we check if we have the data. if not we have to go back up getting data :a
        // then we can return the data!

        let levels = Vec::new();
        let ul = self.upper_level;
        while let Some(upper_level) = &ul {
            levels.push(upper_level);
            // index = upper_level.get_level_down_index(&self, index);
        }

        for i in (1..levels.len()).rev() {
            let ul = levels[i];
            let ll = levels[i - 1];
            let our_index = ul.get_level_down_index(ll, index);
            match our_index {
                Some(new_index) => index = new_index,
                None => // need to get cache line and then return the memory!
            }
        }
    }

    /// returns the time needed to put these bytes
    pub fn put_bytes(&mut self, index: usize, bytes: &[u8]) -> u32 {
        (&mut self.bytes[index..(index + bytes.len())]).put(bytes);
        speed
    }
}
