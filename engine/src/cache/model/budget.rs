#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BudgetWatermark {
    pub soft: usize,
    pub hard: usize,
}

impl BudgetWatermark {
    pub fn new(soft: usize, hard: usize) -> Self {
        assert!(soft <= hard, "soft watermark must be <= hard watermark");
        Self { soft, hard }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CacheBudget {
    pub result_budget_bytes: BudgetWatermark,
    pub texture_budget_bytes: BudgetWatermark,
    pub handle_budget_bytes: BudgetWatermark,
}

impl CacheBudget {
    pub fn new(
        result_budget_bytes: BudgetWatermark,
        texture_budget_bytes: BudgetWatermark,
        handle_budget_bytes: BudgetWatermark,
    ) -> Self {
        Self {
            result_budget_bytes,
            texture_budget_bytes,
            handle_budget_bytes,
        }
    }
}

impl Default for CacheBudget {
    fn default() -> Self {
        Self {
            result_budget_bytes: BudgetWatermark::new(64 * 1024 * 1024, 128 * 1024 * 1024),
            texture_budget_bytes: BudgetWatermark::new(64 * 1024 * 1024, 128 * 1024 * 1024),
            handle_budget_bytes: BudgetWatermark::new(64 * 1024 * 1024, 128 * 1024 * 1024),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BudgetWatermark, CacheBudget};

    #[test]
    fn cache_budget_keeps_three_buckets() {
        let budget = CacheBudget::new(
            BudgetWatermark::new(10, 20),
            BudgetWatermark::new(30, 40),
            BudgetWatermark::new(50, 60),
        );

        assert_eq!(budget.handle_budget_bytes.hard, 60);
    }
}
