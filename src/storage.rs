use std::collections::{HashMap, HashSet};
use std::error::Error;
use serde::export::Formatter;
use super::util::cumulative_weights::CumulativeWeights;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

type Category = String;
type BannerIdx = usize;

const HTML_PREFIX: &str = r#"<html><body><img src=""#;
const HTML_SUFFIX: &str = r#""/></body></html>"#;

pub trait Storage {
    fn add_banner(&mut self, url: String, shows_amount: u32, categories: Vec<Category>) -> Result<(), StoreError>;
    fn get_banner_html(&self, categories: Vec<Category>) -> Option<String>;
}

#[derive(Debug, PartialEq)]
pub enum StoreError {
    IllegalUrl,
    IllegalShowsAmount,
    EmptyCategories,
}

impl Error for StoreError {}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let description = match self {
            StoreError::IllegalUrl => "Illegal URL",
            StoreError::IllegalShowsAmount => "Shows amount must be positive",
            StoreError::EmptyCategories => "Banner must have at least one category",
        };
        write!(f, "{}", description)
    }
}

#[derive(Debug)]
struct Banner {
    url: String,
    shows_amount: u32,
    shows_left: Arc<AtomicU32>,
}

impl Banner {
    fn new(url: String, shows_amount: u32) -> Self {
        Banner {
            url,
            shows_amount,
            shows_left: Arc::new(AtomicU32::new(shows_amount)),
        }
    }

    fn show_html(&self) -> Option<String> {
        let shows_left= &self.shows_left.clone();
        let mut left = shows_left.load(Ordering::SeqCst);

        while left > 0 && shows_left.compare_and_swap(left, left - 1, Ordering::SeqCst) != left {
            left = shows_left.load(Ordering::SeqCst);
        }

        if left == 0 {
            return None
        } else {
            Some(format!("{}{}{}", HTML_PREFIX, self.url, HTML_SUFFIX))
        }
    }

    fn can_show(&self) -> bool {
        let shows_left = &self.shows_left.clone();
        shows_left.load(Ordering::SeqCst) > 0
    }
}

#[derive(Debug)]
pub struct InMemoryStorage {
    banners: Vec<Banner>,
    index: HashMap<Category, Vec<BannerIdx>>,
    cumulative_weights: CumulativeWeights, // weight for banners vector used to weighted selection
}

impl std::fmt::Display for InMemoryStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "InMemoryStore[banners: {}, categories: {}]", self.banners.len(), self.index.len())
    }
}

impl InMemoryStorage {
    pub fn new() -> Self {
        InMemoryStorage {
            banners: Vec::new(),
            index: HashMap::new(),
            cumulative_weights: CumulativeWeights::new(),
        }
    }
}

impl Storage for InMemoryStorage {
    fn add_banner(&mut self, url: String, shows_amount: u32, categories: Vec<String>) -> Result<(), StoreError> {
        if url.is_empty() {
            return Err(StoreError::IllegalUrl);
        }

        if shows_amount == 0 {
            return Err(StoreError::IllegalShowsAmount);
        }

        if categories.is_empty() {
            return Err(StoreError::EmptyCategories);
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

        Ok(())
    }

    fn get_banner_html(&self, categories: Vec<Category>) -> Option<String> {
        match self.filter_by_categories(categories) {
            FilterResult::All => {
                self.show_html_select_all()
            }
            FilterResult::Slice { indexes } => {
                let weights = self.get_cumulative_weights(&indexes);
                self.show_html(&weights)
            }
        }
    }
}

enum FilterResult<'a> {
    All,
    Slice { indexes: HashSet<&'a BannerIdx> },
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
        return FilterResult::Slice { indexes };
    }

    fn get_cumulative_weights(&self, indexes: &HashSet<&usize>) -> CumulativeWeights {
        let mut weights = CumulativeWeights::with_projection();
        for &idx in indexes {
            let banner = &self.banners[*idx];
            weights.add_weight_for_idx(banner.shows_amount, *idx);
        }
        weights
    }

    fn show_html(&self, weights: &CumulativeWeights) -> Option<String> {
        weights.select_uniformly()
            .and_then(|idx| self.banners.get(idx))
            .and_then(|banner| banner.show_html())
    }

    fn show_html_select_all(&self) -> Option<String> {
        self.cumulative_weights.select_uniformly()
            .and_then(|idx| self.banners.get(idx))
            .and_then(|banner| banner.show_html())
    }
}

#[cfg(test)]
mod tests {
    use crate::storage::{InMemoryStorage, Storage, StoreError};

    #[test]
    fn empty_storage() {
        // arrange
        let storage = InMemoryStorage::new();

        // act
        let html = storage.get_banner_html(vec![]);

        // assert
        assert_eq!(html, None)
    }

    #[test]
    fn store_errors() {
        // arrange
        let mut storage = InMemoryStorage::new();

        // act
        let illegal_url_res = storage.add_banner("".to_string(), 1, vec!["cat".to_string()]);
        let illegal_shows_amount_res = storage.add_banner("some".to_string(), 0, vec!["cat".to_string()]);
        let illegal_categories_res = storage.add_banner("some".to_string(), 1, vec![]);

        // assert
        assert_eq!(illegal_url_res, Err(StoreError::IllegalUrl));
        assert_eq!(illegal_shows_amount_res, Err(StoreError::IllegalShowsAmount));
        assert_eq!(illegal_categories_res, Err(StoreError::EmptyCategories));
    }

    #[test]
    fn single_banner_store() {
        // arrange
        let mut storage = InMemoryStorage::new();
        let url = "http://example.com/1.jpg";
        let categories = vec!["example".to_string()];

        // act
        let store_res = storage.add_banner(String::from(url), 1, categories);
        let html = storage.get_banner_html(vec![]);

        // assert
        assert_eq!(store_res, Ok(()));
        assert!(html.is_some());
        assert!(html.unwrap().contains(url));
    }

    #[test]
    fn shows_amount_decreased() {
        // arrange
        let mut storage = InMemoryStorage::new();
        let url = "http://example.com/1.jpg";
        let categories = vec!["example".to_string()];

        // act
        let store_res = storage.add_banner(String::from(url), 2, categories);
        let html = storage.get_banner_html(vec![]);
        let html2 = storage.get_banner_html(vec![]);
        let html3 = storage.get_banner_html(vec![]);

        // assert
        assert_eq!(store_res, Ok(()));
        assert!(html.is_some());
        assert!(html.unwrap().contains(url));
        assert!(html2.is_some());
        assert!(html2.unwrap().contains(url));
        assert!(html3.is_none());
    }

    #[test]
    fn filter_by_categories() {
        // arrange
        let mut storage = InMemoryStorage::new();
        let url1 = "http://example.com/1.jpg".to_string();
        let url2 = "http://example.com/2.jpg".to_string();

        // act
        let store_res1 = storage.add_banner(url1.clone(), 2, categories(&["cat1", "cat2"]));
        let store_res2 = storage.add_banner(url2.clone(), 1, categories(&["cat3"]));
        let html1 = storage.get_banner_html(categories(&["cat1"]));
        let html2 = storage.get_banner_html(categories(&["cat2"]));
        let html3 = storage.get_banner_html(categories(&["cat1"]));
        let html4 = storage.get_banner_html(categories(&["cat3"]));
        let html5 = storage.get_banner_html(categories(&["cat3"]));

        // assert
        assert_eq!(store_res1, Ok(()));
        assert_eq!(store_res2, Ok(()));
        assert_html(html1, &url1);
        assert_html(html2, &url1);
        assert_no_html(html3);
        assert_html(html4, &url2);
        assert_no_html(html5);
    }

    #[test]
    fn filter_by_unknown_category() {
        // arrange
        let mut storage = InMemoryStorage::new();

        // act
        let store_res = storage.add_banner("url".to_string(), 1, categories(&["example"]));
        let html = storage.get_banner_html(categories(&["unknown"]));

        // assert
        assert_eq!(store_res, Ok(()));
        assert_no_html(html);
    }

    #[test]
    fn filter_by_common_category() {
        // arrange
        let mut storage = InMemoryStorage::new();

        // act
        storage.add_banner("url1".to_string(), 1, categories(&["cat1"])).unwrap();
        storage.add_banner("url2".to_string(), 1, categories(&["cat2"])).unwrap();
        let html1 = storage.get_banner_html(categories(&["cat1", "cat2"]));
        let html2 = storage.get_banner_html(categories(&["cat1", "cat2"]));
        let html3 = storage.get_banner_html(categories(&["cat1", "cat2"]));

        // assert
        assert_html_one_of(html1, &["url1", "url2"]);
        assert_html_one_of(html2, &["url1", "url2"]);
        assert_no_html(html3);
    }

    fn categories(cats: &[&str]) -> Vec<String> {
        cats.iter()
            .map(|s| s.to_string())
            .collect()
    }

    fn assert_html(html: Option<String>, url: &String) {
        assert!(html.is_some());
        assert!(html.unwrap().contains(url));
    }

    fn assert_html_one_of(html: Option<String>, urls: &[&str]) {
        assert!(html.is_some());
        let res = &html.unwrap();
        let contains_any = urls.iter()
            .any(|url| res.contains(url));
        assert!(contains_any);
    }

    fn assert_no_html(html: Option<String>) {
        assert!(html.is_none());
    }
}