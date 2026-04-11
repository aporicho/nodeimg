use crate::cache::model::BudgetWatermark;
use crate::cache::model::ResultEntry;

use super::lookup::ResultStore;

impl ResultStore {
    pub fn evict_to_budget(&self, budget: BudgetWatermark) -> Vec<ResultEntry> {
        let mut evicted = Vec::new();

        loop {
            if self.total_bytes() <= budget.soft {
                break;
            }

            let Some(key) = self.pop_lru_key() else {
                break;
            };

            if let Some(entry) = self.remove(&key) {
                evicted.push(entry);
            }
        }

        evicted
    }
}
