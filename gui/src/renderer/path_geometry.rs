use lyon::math::{point, vector, Angle};
use lyon::path::traits::SvgPathBuilder;
use lyon::path::{ArcFlags, Path};
use lyon::tessellation::FillRule as LyonFillRule;

use crate::geometry::{Point, Rect};
use crate::paint::{FillRule, PathCommand, PathData};

pub(super) const DEFAULT_CORNER_SMOOTHING: f32 = 0.6;
const CIRCLE_KAPPA: f32 = 0.552_284_8;

struct CornerParams {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    p: f32,
    arc_section_length: f32,
    corner_radius: f32,
}

fn get_corner_params(
    corner_radius: f32,
    mut corner_smoothing: f32,
    rounding_and_smoothing_budget: f32,
) -> CornerParams {
    let mut p = (1.0 + corner_smoothing) * corner_radius;

    if p > rounding_and_smoothing_budget {
        let max_smoothing = rounding_and_smoothing_budget / corner_radius - 1.0;
        corner_smoothing = corner_smoothing.min(max_smoothing);
        p = p.min(rounding_and_smoothing_budget);
    }

    let arc_measure = 90.0 * (1.0 - corner_smoothing);
    let arc_section_length =
        (arc_measure / 2.0).to_radians().sin() * corner_radius * std::f32::consts::SQRT_2;

    let angle_alpha = (90.0 - arc_measure) / 2.0;
    let p3_to_p4_distance = corner_radius * (angle_alpha / 2.0).to_radians().tan();

    let angle_beta = 45.0 * corner_smoothing;
    let c = p3_to_p4_distance * angle_beta.to_radians().cos();
    let d = c * angle_beta.to_radians().tan();

    let b = (p - arc_section_length - c - d) / 3.0;
    let a = 2.0 * b;

    CornerParams {
        a,
        b,
        c,
        d,
        p,
        arc_section_length,
        corner_radius,
    }
}

pub(super) fn build_rounded_rect_path(rect: Rect, radius: [f32; 4], smoothing: f32) -> Path {
    let w = rect.w;
    let h = rect.h;
    let budget = w.min(h) / 2.0;

    let tr = get_corner_params(radius[1].min(budget), smoothing, budget);
    let br = get_corner_params(radius[2].min(budget), smoothing, budget);
    let bl = get_corner_params(radius[3].min(budget), smoothing, budget);
    let tl = get_corner_params(radius[0].min(budget), smoothing, budget);

    let x = rect.x;
    let y = rect.y;
    let flags = ArcFlags {
        large_arc: false,
        sweep: true,
    };

    let mut b = Path::builder().with_svg();
    b.move_to(point(x + w - tr.p, y));

    if tr.corner_radius > 0.0 {
        let abc = tr.a + tr.b + tr.c;
        let al = tr.arc_section_length;
        let r = tr.corner_radius;
        b.relative_cubic_bezier_to(
            vector(tr.a, 0.0),
            vector(tr.a + tr.b, 0.0),
            vector(abc, tr.d),
        );
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(al, al));
        b.relative_cubic_bezier_to(
            vector(tr.d, tr.c),
            vector(tr.d, tr.b + tr.c),
            vector(tr.d, abc),
        );
    }

    b.line_to(point(x + w, y + h - br.p));

    if br.corner_radius > 0.0 {
        let abc = br.a + br.b + br.c;
        let al = br.arc_section_length;
        let r = br.corner_radius;
        b.relative_cubic_bezier_to(
            vector(0.0, br.a),
            vector(0.0, br.a + br.b),
            vector(-br.d, abc),
        );
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(-al, al));
        b.relative_cubic_bezier_to(
            vector(-br.c, br.d),
            vector(-(br.b + br.c), br.d),
            vector(-abc, br.d),
        );
    }

    b.line_to(point(x + bl.p, y + h));

    if bl.corner_radius > 0.0 {
        let abc = bl.a + bl.b + bl.c;
        let al = bl.arc_section_length;
        let r = bl.corner_radius;
        b.relative_cubic_bezier_to(
            vector(-bl.a, 0.0),
            vector(-(bl.a + bl.b), 0.0),
            vector(-abc, -bl.d),
        );
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(-al, -al));
        b.relative_cubic_bezier_to(
            vector(-bl.d, -bl.c),
            vector(-bl.d, -(bl.b + bl.c)),
            vector(-bl.d, -abc),
        );
    }

    b.line_to(point(x, y + tl.p));

    if tl.corner_radius > 0.0 {
        let abc = tl.a + tl.b + tl.c;
        let al = tl.arc_section_length;
        let r = tl.corner_radius;
        b.relative_cubic_bezier_to(
            vector(0.0, -tl.a),
            vector(0.0, -(tl.a + tl.b)),
            vector(tl.d, -abc),
        );
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(al, -al));
        b.relative_cubic_bezier_to(
            vector(tl.c, -tl.d),
            vector(tl.b + tl.c, -tl.d),
            vector(abc, -tl.d),
        );
    }

    b.close();
    b.build()
}

pub(super) fn build_lyon_path(data: &PathData) -> Path {
    let mut builder = Path::builder();
    let mut open = false;

    for command in &data.commands {
        match *command {
            PathCommand::MoveTo(to) => {
                if open {
                    builder.end(false);
                }
                builder.begin(point(to.x, to.y));
                open = true;
            }
            PathCommand::LineTo(to) => {
                if open {
                    builder.line_to(point(to.x, to.y));
                }
            }
            PathCommand::QuadTo(ctrl, to) => {
                if open {
                    builder.quadratic_bezier_to(point(ctrl.x, ctrl.y), point(to.x, to.y));
                }
            }
            PathCommand::CubicTo(ctrl1, ctrl2, to) => {
                if open {
                    builder.cubic_bezier_to(
                        point(ctrl1.x, ctrl1.y),
                        point(ctrl2.x, ctrl2.y),
                        point(to.x, to.y),
                    );
                }
            }
            PathCommand::Close => {
                if open {
                    builder.close();
                    open = false;
                }
            }
        }
    }

    if open {
        builder.end(false);
    }

    builder.build()
}

pub(super) fn circle_path_data(center: Point, radius: f32) -> PathData {
    let r = radius.max(0.0);
    let c = r * CIRCLE_KAPPA;
    PathData::new()
        .move_to(Point {
            x: center.x + r,
            y: center.y,
        })
        .cubic_to(
            Point {
                x: center.x + r,
                y: center.y + c,
            },
            Point {
                x: center.x + c,
                y: center.y + r,
            },
            Point {
                x: center.x,
                y: center.y + r,
            },
        )
        .cubic_to(
            Point {
                x: center.x - c,
                y: center.y + r,
            },
            Point {
                x: center.x - r,
                y: center.y + c,
            },
            Point {
                x: center.x - r,
                y: center.y,
            },
        )
        .cubic_to(
            Point {
                x: center.x - r,
                y: center.y - c,
            },
            Point {
                x: center.x - c,
                y: center.y - r,
            },
            Point {
                x: center.x,
                y: center.y - r,
            },
        )
        .cubic_to(
            Point {
                x: center.x + c,
                y: center.y - r,
            },
            Point {
                x: center.x + r,
                y: center.y - c,
            },
            Point {
                x: center.x + r,
                y: center.y,
            },
        )
        .close()
}

pub(super) fn lyon_fill_rule(rule: FillRule) -> LyonFillRule {
    match rule {
        FillRule::NonZero => LyonFillRule::NonZero,
        FillRule::EvenOdd => LyonFillRule::EvenOdd,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_path_closes_four_cubics() {
        let data = circle_path_data(Point { x: 10.0, y: 20.0 }, 5.0);

        assert_eq!(data.commands.len(), 6);
        assert!(matches!(data.commands.last(), Some(PathCommand::Close)));
    }

    #[test]
    fn rounded_rect_path_has_geometry() {
        let path = build_rounded_rect_path(
            Rect {
                x: 0.0,
                y: 0.0,
                w: 10.0,
                h: 10.0,
            },
            [2.0; 4],
            DEFAULT_CORNER_SMOOTHING,
        );

        assert!(path.iter().next().is_some());
    }
}
