use std::collections::{HashMap, HashSet};
use rand::prelude::*;

type Category = String;

const HTML_PREFIX: &str = r#"<html><body><img src=""#;
const HTML_SUFFIX: &str = r#""/></body></html>"#;

trait Storage {
    fn add_banner(&mut self, url: String, shows_amount: u32, categories: Vec<Category>);
    fn get_banner_html(&mut self, categories: Vec<Category>) -> Option<String>;
}

#[derive(Debug)]
struct Banner {
    url: String,
    shows_amount: u32,
    shows_left: u32,
}

impl Banner {
    fn new(url: String, shows_amount: u32) -> Self {
        Banner {
            url,
            shows_amount,
            shows_left: shows_amount
        }
    }

    fn show_html(&mut self) -> Option<String> {
        if !self.can_show() {
            return None
        }

        self.shows_left -= 1;
        Some(format!("{}{}{}", HTML_PREFIX, self.url, HTML_SUFFIX))
    }

    fn can_show(&self) -> bool {
        self.shows_left > 0
    }
}

type BannerIdx = usize;

#[derive(Debug)]
struct InMemoryStorage {
    banners: Vec<Banner>,
    index: HashMap<Category, Vec<BannerIdx>>,
    cumulative_weights: CumulativeWeights
}

#[derive(Debug)]
struct CumulativeWeights {
    weights: Vec<u64>,
    idx_projection: IdxProjection,
}

impl CumulativeWeights {
    fn new() -> Self {
        CumulativeWeights {
            weights: Vec::new(),
            idx_projection: IdxProjection::AsIs
        }
    }

    fn with_projection() -> Self {
        CumulativeWeights {
            weights: Vec::new(),
            idx_projection: IdxProjection::Specific(Vec::new()),
        }
    }

    fn add_weight(&mut self, weight: u32) {
        let last_weight = self.weights.last().copied().unwrap_or(0);
        self.weights.push(last_weight + weight as u64)
    }

    fn add_weight_for_idx(&mut self, weight: u32, idx: usize) {
        match self.idx_projection {
            IdxProjection::Specific(ref mut p) => {
                p.push(idx);
                self.add_weight(weight);
            }
            _ => panic!("Can't add projection")
        }
    }

    fn select_uniformly(&self) -> Option<usize> {
        if self.weights.is_empty() {
            return None
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

#[derive(Debug)]
enum IdxProjection {
    AsIs,
    Specific(Vec<usize>)
}

impl IdxProjection {
    fn project(&self, idx: usize) -> usize {
        match self {
            IdxProjection::AsIs => idx,
            IdxProjection::Specific(p) => p[idx]
        }
    }
}

impl InMemoryStorage {
    fn new() -> Self {
        InMemoryStorage {
            banners: Vec::new(),
            index: HashMap::new(),
            cumulative_weights: CumulativeWeights::new(),
        }
    }
}

impl Storage for InMemoryStorage {
    fn add_banner(&mut self, url: String, shows_amount: u32, categories: Vec<String>) {
        if url.is_empty() || shows_amount == 0 || categories.is_empty() {
            return;
        }

        let banner = Banner::new(url, shows_amount);
        let banner_idx = self.banners.len();
        self.banners.push(banner);

        for category in categories {
            self.index.entry(category)
                .and_modify(|indexes| indexes.push(banner_idx))
                .or_insert_with(|| vec![banner_idx]);
        }

        self.cumulative_weights.add_weight(shows_amount);
    }

    fn get_banner_html(&mut self, categories: Vec<Category>) -> Option<String> {
        match self.filter_by_categories(categories) {
            FilterResult::All => {
                self.show_html_select_all()
            },
            FilterResult::Slice { indexes} => {
                let weights = self.get_cumulative_weights(&indexes);
                self.show_html(&weights)
            }
        }
    }
}

enum FilterResult<'a> {
    All,
    Slice { indexes: HashSet<&'a BannerIdx> }
}

impl InMemoryStorage {
    fn filter_by_categories(&self, categories: Vec<String>) -> FilterResult {
        if categories.is_empty() {
            return FilterResult::All;
        }

        let indexes = categories.iter()
            .filter_map(|category| self.index.get(category))
            .flatten()
            .filter(|&idx| self.banners[*idx].can_show())
            .collect::<HashSet<&BannerIdx>>();
        return FilterResult::Slice { indexes }
    }

    fn get_cumulative_weights(&self, indexes: &HashSet<&usize>) -> CumulativeWeights {
        let mut weights = CumulativeWeights::with_projection();
        for &idx in indexes {
            let banner = &self.banners[*idx];
            weights.add_weight_for_idx(banner.shows_amount, *idx);
        }
        weights
    }

    fn show_html(&mut self, weights: &CumulativeWeights) -> Option<String> {
        weights.select_uniformly()
            .and_then(|idx| self.banners.get_mut(idx))
            .and_then(|banner| banner.show_html())
    }

    fn show_html_select_all(&mut self) -> Option<String> {
        self.cumulative_weights.select_uniformly()
            .and_then(|idx| self.banners.get_mut(idx))
            .and_then(|banner| banner.show_html())
    }
}
