use lyon::math::{point, vector, Angle};
use lyon::path::traits::SvgPathBuilder;
use lyon::path::{ArcFlags, Path};

use super::types::Rect;


/// 默认 corner smoothing（iOS 风格）
pub const DEFAULT_CORNER_SMOOTHING: f32 = 0.6;

// ── Figma cornerSmoothing 算法 ──

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

/// 生成 Figma 风格的圆角矩形路径
///
/// 严格翻译自 figma-squircle 的 getSVGPathFromPathParams + drawCornerPath。
/// 使用 lyon 的 SVG builder，直接映射 SVG 相对命令（c, a）。
pub fn build_rounded_rect_path(rect: Rect, radius: [f32; 4], smoothing: f32) -> Path {
    let w = rect.w;
    let h = rect.h;
    let budget = w.min(h) / 2.0;

    let tr = get_corner_params(radius[1].min(budget), smoothing, budget);
    let br = get_corner_params(radius[2].min(budget), smoothing, budget);
    let bl = get_corner_params(radius[3].min(budget), smoothing, budget);
    let tl = get_corner_params(radius[0].min(budget), smoothing, budget);

    let x = rect.x;
    let y = rect.y;
    let flags = ArcFlags { large_arc: false, sweep: true };

    let mut b = Path::builder().with_svg();

    // M (w - tr.p) 0
    b.move_to(point(x + w - tr.p, y));

    // 右上角: c a 0 (a+b) 0 (a+b+c) d | a R R 0 0 1 al al | c d c d (b+c) d (a+b+c)
    if tr.corner_radius > 0.0 {
        let abc = tr.a + tr.b + tr.c;
        let al = tr.arc_section_length;
        let r = tr.corner_radius;
        b.relative_cubic_bezier_to(vector(tr.a, 0.0), vector(tr.a + tr.b, 0.0), vector(abc, tr.d));
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(al, al));
        b.relative_cubic_bezier_to(vector(tr.d, tr.c), vector(tr.d, tr.b + tr.c), vector(tr.d, abc));
    }

    // L w (h - br.p)
    b.line_to(point(x + w, y + h - br.p));

    // 右下角: c 0 a 0 (a+b) -d (a+b+c) | a R R 0 0 1 -al al | c -c d -(b+c) d -(a+b+c) d
    if br.corner_radius > 0.0 {
        let abc = br.a + br.b + br.c;
        let al = br.arc_section_length;
        let r = br.corner_radius;
        b.relative_cubic_bezier_to(vector(0.0, br.a), vector(0.0, br.a + br.b), vector(-br.d, abc));
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(-al, al));
        b.relative_cubic_bezier_to(vector(-br.c, br.d), vector(-(br.b + br.c), br.d), vector(-abc, br.d));
    }

    // L bl.p h
    b.line_to(point(x + bl.p, y + h));

    // 左下角: c -a 0 -(a+b) 0 -(a+b+c) -d | a R R 0 0 1 -al -al | c -d -c -d -(b+c) -d -(a+b+c)
    if bl.corner_radius > 0.0 {
        let abc = bl.a + bl.b + bl.c;
        let al = bl.arc_section_length;
        let r = bl.corner_radius;
        b.relative_cubic_bezier_to(vector(-bl.a, 0.0), vector(-(bl.a + bl.b), 0.0), vector(-abc, -bl.d));
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(-al, -al));
        b.relative_cubic_bezier_to(vector(-bl.d, -bl.c), vector(-bl.d, -(bl.b + bl.c)), vector(-bl.d, -abc));
    }

    // L 0 tl.p
    b.line_to(point(x, y + tl.p));

    // 左上角: c 0 -a 0 -(a+b) d -(a+b+c) | a R R 0 0 1 al -al | c c -d (b+c) -d (a+b+c) -d
    if tl.corner_radius > 0.0 {
        let abc = tl.a + tl.b + tl.c;
        let al = tl.arc_section_length;
        let r = tl.corner_radius;
        b.relative_cubic_bezier_to(vector(0.0, -tl.a), vector(0.0, -(tl.a + tl.b)), vector(tl.d, -abc));
        b.relative_arc_to(vector(r, r), Angle::zero(), flags, vector(al, -al));
        b.relative_cubic_bezier_to(vector(tl.c, -tl.d), vector(tl.b + tl.c, -tl.d), vector(abc, -tl.d));
    }

    // Z
    b.close();
    b.build()
}
