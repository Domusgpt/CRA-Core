use crate::hash::compute_hash;

pub struct HashChain {
    pub previous_hash: String,
}

impl HashChain {
    pub fn new(seed: &str) -> Self {
        Self {
            previous_hash: seed.to_string(),
        }
    }

    pub fn add(&mut self, data: &[u8]) -> String {
        let mut combined = Vec::new();
        combined.extend_from_slice(self.previous_hash.as_bytes());
        combined.extend_from_slice(data);

        let new_hash = compute_hash(&combined);
        self.previous_hash = new_hash.clone();
        new_hash
    }
}
