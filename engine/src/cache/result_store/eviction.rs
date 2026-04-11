use crate::cache::model::BudgetWatermark;

use super::lookup::ResultStore;

impl ResultStore {
    pub fn evict_to_budget(&self, budget: BudgetWatermark) -> usize {
        let mut evicted = 0;

        loop {
            if self.total_bytes() <= budget.soft {
                break;
            }

            let Some(key) = self.pop_lru_key() else {
                break;
            };

            if self.remove(&key).is_some() {
                evicted += 1;
            }
        }

        evicted
    }
}
