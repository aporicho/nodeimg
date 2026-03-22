/// Describes the valid range/options for a parameter value.
#[derive(Clone, Debug)]
pub enum Constraint {
    /// No constraint.
    None,
    /// Numeric range (inclusive).
    Range { min: f64, max: f64 },
    /// Enumerated options: list of (label, value) pairs.
    Enum { options: Vec<(String, String)> },
    /// File path with extension filters.
    FilePath { filters: Vec<String> },
}

/// Unique identifier for a constraint type (for widget matching).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ConstraintType {
    None,
    Range,
    Enum,
    FilePath,
}

impl Constraint {
    /// Returns the constraint type identifier for widget matching.
    pub fn constraint_type(&self) -> ConstraintType {
        match self {
            Constraint::None => ConstraintType::None,
            Constraint::Range { .. } => ConstraintType::Range,
            Constraint::Enum { .. } => ConstraintType::Enum,
            Constraint::FilePath { .. } => ConstraintType::FilePath,
        }
    }

    /// Validates a float value against this constraint.
    pub fn validate_f64(&self, value: f64) -> bool {
        match self {
            Constraint::Range { min, max } => value >= *min && value <= *max,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_type() {
        let c = Constraint::Range { min: 0.0, max: 1.0 };
        assert_eq!(c.constraint_type(), ConstraintType::Range);

        let c = Constraint::Enum {
            options: vec![("A".into(), "a".into())],
        };
        assert_eq!(c.constraint_type(), ConstraintType::Enum);

        let c = Constraint::None;
        assert_eq!(c.constraint_type(), ConstraintType::None);
    }

    #[test]
    fn test_range_validation() {
        let c = Constraint::Range {
            min: -1.0,
            max: 1.0,
        };
        assert!(c.validate_f64(0.0));
        assert!(c.validate_f64(-1.0));
        assert!(c.validate_f64(1.0));
        assert!(!c.validate_f64(1.1));
        assert!(!c.validate_f64(-1.1));
    }
}
