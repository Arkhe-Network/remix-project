use blake3::Hasher;

pub type MerkleHash = [u8; 32];

#[derive(Debug, Clone)]
pub struct MerkleTree {
    levels: Vec<Vec<MerkleHash>>,
    leaf_count: usize,
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

impl MerkleTree {
    pub fn new() -> Self {
        Self { levels: Vec::new(), leaf_count: 0 }
    }

    pub fn append_leaf(&mut self, leaf: MerkleHash) {
        self.leaf_count += 1;

        if self.levels.is_empty() {
            self.levels.push(vec![leaf]);
            return;
        }

        let mut current_hash = leaf;
        let mut index = self.leaf_count - 1;
        let mut level = 0;

        while index % 2 == 1 || level < self.levels.len() {
            if index % 2 == 1 {
                let sibling = self.levels[level][index - 1];
                let mut hasher = Hasher::new();
                hasher.update(&sibling);
                hasher.update(&current_hash);
                current_hash = hasher.finalize().into();
            } else {
                if level + 1 >= self.levels.len() {
                    self.levels.push(vec![]);
                }
            }

            if level < self.levels.len() {
                let next_index = index / 2;
                if next_index < self.levels[level + 1].len() {
                    self.levels[level + 1][next_index] = current_hash;
                } else {
                    self.levels[level + 1].push(current_hash);
                }
            }

            index /= 2;
            level += 1;
        }

        self.levels[0].push(leaf);
    }

    pub fn root(&self) -> Option<MerkleHash> {
        self.levels.last().and_then(|last| last.first()).copied()
    }
}
