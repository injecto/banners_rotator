use rand::prelude::*;

#[derive(Debug)]
pub struct CumulativeWeights {
    weights: Vec<u64>,
    idx_projection: IdxProjection,
}

#[derive(Debug, PartialEq)]
enum IdxProjection {
    AsIs,

    ///
    /// vec index mapped to vec[index] value
    ///
    Specific(Vec<usize>),
}

impl CumulativeWeights {
    pub(crate) fn new() -> Self {
        CumulativeWeights {
            weights: Vec::new(),
            idx_projection: IdxProjection::AsIs,
        }
    }

    pub(crate) fn with_projection() -> Self {
        CumulativeWeights {
            weights: Vec::new(),
            idx_projection: IdxProjection::Specific(Vec::new()),
        }
    }

    pub(crate) fn add_weight(&mut self, weight: u32) {
        if self.idx_projection != IdxProjection::AsIs {
            panic!("Can't add weight without index projetion")
        }
        let last_weight = self.weights.last().copied().unwrap_or(0);
        self.weights.push(last_weight + weight as u64)
    }

    pub(crate) fn add_weight_for_idx(&mut self, weight: u32, idx: usize) {
        match self.idx_projection {
            IdxProjection::Specific(ref mut p) => {
                p.push(idx);
                self.add_weight(weight);
            }
            _ => panic!("Can't add projection")
        }
    }

    pub(crate) fn select_uniformly(&self) -> Option<usize> {
        if self.weights.is_empty() {
            return None;
        }

        let idx = if self.weights.len() == 1 {
            0
        } else {
            let max = self.weights.last().unwrap();
            let rnd = thread_rng().gen_range(0u64, max + 1);

            match self.weights.binary_search(&rnd) {
                Ok(exact_idx) => exact_idx,
                Err(insert_idx) => insert_idx,
            }
        };

        Some(self.idx_projection.project(idx))
    }
}

impl IdxProjection {
    fn project(&self, idx: usize) -> usize {
        match self {
            IdxProjection::AsIs => idx,
            IdxProjection::Specific(p) => p[idx]
        }
    }
}
