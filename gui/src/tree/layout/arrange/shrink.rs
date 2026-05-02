use super::axis::is_main_fill;
use super::child_style::FlexChildStyle;

pub(super) fn resolve_shrink_main_sizes(
    base_sizes: &[f32],
    styles: &[FlexChildStyle],
    is_column: bool,
    main_available: f32,
    total_gap: f32,
) -> Vec<f32> {
    let target_children_main = (main_available - total_gap).max(0.0);
    let base_children_main = base_sizes.iter().sum::<f32>();
    let overflow = (base_children_main - target_children_main).max(0.0);
    if overflow <= 0.0 {
        return base_sizes.to_vec();
    }

    let shrink_factors: Vec<f32> = base_sizes
        .iter()
        .zip(styles.iter())
        .map(|(base, style)| effective_shrink_factor(*style, is_column) * *base)
        .collect();
    let total_shrink_factor = shrink_factors.iter().sum::<f32>();
    if total_shrink_factor <= 0.0 {
        return base_sizes.to_vec();
    }

    base_sizes
        .iter()
        .zip(styles.iter())
        .zip(shrink_factors.iter())
        .map(|((base, style), factor)| {
            let shrink = overflow * *factor / total_shrink_factor;
            let min_main = if is_column {
                style.min_height
            } else {
                style.min_width
            };
            let max_main = if is_column {
                style.max_height
            } else {
                style.max_width
            };
            (*base - shrink).clamp(min_main, max_main)
        })
        .collect()
}

fn effective_shrink_factor(style: FlexChildStyle, is_column: bool) -> f32 {
    let explicit = style.shrink.max(0.0);
    if explicit > 0.0 {
        explicit
    } else if is_main_fill(style, is_column) {
        1.0
    } else {
        0.0
    }
}
