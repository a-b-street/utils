use geo::{Coord, EuclideanDistance, Line, LineString, Polygon};

// If width is negative, shifts left
pub fn shift_linestring(
    input: &LineString,
    width: f64,
    miter_threshold: f64,
) -> Option<LineString> {
    if input.0.len() == 2 {
        return Some(shift_line(input.lines().next().unwrap(), width).into());
    }

    let mut result: Vec<Coord> = Vec::new();

    let mut pt3_idx = 2;
    let mut pt1_raw = input.0[0];
    let mut pt2_raw = input.0[1];

    loop {
        let pt3_raw = input.0[pt3_idx];

        let l1 = shift_line(Line::new(pt1_raw, pt2_raw), width);
        let l2 = shift_line(Line::new(pt2_raw, pt3_raw), width);

        if pt3_idx == 2 {
            result.push(l1.start);
        }

        if let Some(pt2_shift) = infinite_lines_intersection(l1, l2) {
            // Miter caps sometimes explode out to infinity. Hackily work around this.
            let dist_away = l1.end.euclidean_distance(&pt2_shift);
            if dist_away < miter_threshold {
                result.push(pt2_shift);
            } else {
                result.push(l1.end);
            }
        } else {
            // When the lines are perfectly parallel, it means pt2_shift_1st == pt2_shift_2nd
            // and the original geometry is redundant.
            result.push(l1.end);
        }
        if pt3_idx == input.0.len() - 1 {
            result.push(l2.end);
            break;
        }

        pt1_raw = pt2_raw;
        pt2_raw = pt3_raw;
        pt3_idx += 1;
    }

    Some(LineString::new(result))
}

/// `center` represents some center, with `total_width`. Logically this shifts left by
/// `total_width / 2`, then right by `width_from_left_side`, but without exasperating sharp
/// bends.
// TODO suspect
pub fn shift_from_center(
    center: &LineString,
    total_width: f64,
    width_from_left_side: f64,
    miter_threshold: f64,
) -> Option<LineString> {
    let half_width = total_width / 2.0;
    shift_linestring(center, width_from_left_side - half_width, miter_threshold)
}

fn line_angle_degrees(line: Line) -> f64 {
    line.dy().atan2(line.dx()).to_degrees()
}

fn project_away(pt: Coord, angle_degrees: f64, distance: f64) -> Coord {
    let (sin, cos) = angle_degrees.to_radians().sin_cos();
    Coord {
        x: pt.x + distance * cos,
        y: pt.y + distance * sin,
    }
}

fn shift_line(line: Line, width: f64) -> Line {
    let angle = line_angle_degrees(line) + if width < 0.0 { -90.0 } else { 90.0 };
    Line::new(
        project_away(line.start, angle, width),
        project_away(line.end, angle, width),
    )
}

// https://stackoverflow.com/a/565282 by way of
// https://github.com/ucarion/line_intersection/blob/master/src/lib.rs
fn infinite_lines_intersection(line1: Line, line2: Line) -> Option<Coord> {
    fn cross(a: (f64, f64), b: (f64, f64)) -> f64 {
        a.0 * b.1 - a.1 * b.0
    }

    let p = line1.start;
    let q = line2.start;
    let r = (line1.end.x - line1.start.x, line1.end.y - line1.start.y);
    let s = (line2.end.x - line2.start.x, line2.end.y - line2.start.y);

    let r_cross_s = cross(r, s);
    let q_minus_p = (q.x - p.x, q.y - p.y);
    //let q_minus_p_cross_r = cross(q_minus_p, r);

    if r_cross_s == 0.0 {
        // Parallel
        None
    } else {
        let t = cross(q_minus_p, (s.0 / r_cross_s, s.1 / r_cross_s));
        //let u = cross(q_minus_p, Pt2D::new(r.x() / r_cross_s, r.y() / r_cross_s));
        Some(Coord {
            x: p.x + t * r.0,
            y: p.y + t * r.1,
        })
    }
}

/*fn shift_with_corrections(&self, width: Distance) -> Result<PolyLine> {
    let raw = self.shift_with_sharp_angles(width, MITER_THRESHOLD)?;
    let result = PolyLine::deduping_new(raw)?;
    if result.pts.len() == self.pts.len() {
        fix_angles(self, result)
    } else {
        Ok(result)
    }
}*/

/*fn fix_angles(orig: &PolyLine, result: PolyLine) -> Result<PolyLine> {
    let mut pts = result.pts.clone();

    // Check that the angles roughly match up between the original and shifted line
    for (idx, (orig_l, shifted_l)) in orig.lines().zip(result.lines()).enumerate() {
        let orig_angle = orig_l.angle();
        let shifted_angle = shifted_l.angle();

        if !orig_angle.approx_eq(shifted_angle, 1.0) {
            // When this happens, the rotation is usually right around 180 -- so try swapping
            // the points!
            pts.swap(idx, idx + 1);
            // TODO Start the fixing over. but make sure we won't infinite loop...
            //return fix_angles(orig, result);
        }
    }

    // When we swap points, length of the entire PolyLine may change! Recalculating is vital.
    PolyLine::new(pts)
}
*/

pub fn buffer_linestring(
    linestring: &LineString,
    left_meters: f64,
    right_meters: f64,
    miter_threshold: f64,
) -> Option<Polygon> {
    assert!(left_meters >= 0.0);
    assert!(right_meters >= 0.0);
    let left = shift_linestring(linestring, -left_meters, miter_threshold)?;
    let right = shift_linestring(linestring, right_meters, miter_threshold)?;
    // Make a polygon by gluing these points together
    let mut pts = left.0;
    pts.reverse();
    pts.extend(right.0);
    Some(Polygon::new(LineString(pts), Vec::new()))
}
