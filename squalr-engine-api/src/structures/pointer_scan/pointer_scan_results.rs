use crate::structures::pointer_scan::pointer_scan_result::PointerScanResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PointerScanResults {
    results: Vec<PointerScanResult>,
    page_size: u64,
}

impl PointerScanResults {
    pub fn new(results: Vec<PointerScanResult>, page_size: u64) -> Self {
        Self { results, page_size }
    }

    pub fn get_results(&self) -> &Vec<PointerScanResult> {
        &self.results
    }

    pub fn set_results(&mut self, results: Vec<PointerScanResult>) {
        self.results = results;
    }

    pub fn get_page_size(&self) -> u64 {
        self.page_size
    }

    pub fn get_result_count(&self) -> u64 {
        self.results.len() as u64
    }

    pub fn get_last_page_index(&self) -> u64 {
        let page_size = self.page_size.max(1);
        let result_count = self.get_result_count();
        if result_count == 0 {
            0
        } else {
            (result_count.saturating_sub(1)) / page_size
        }
    }

    pub fn query_page(&self, page_index: u64) -> Vec<PointerScanResult> {
        let page_size = self.page_size.max(1) as usize;
        let start = page_index.saturating_mul(page_size as u64) as usize;
        let end = start.saturating_add(page_size).min(self.results.len());
        self.results[start..end].to_vec()
    }
}
