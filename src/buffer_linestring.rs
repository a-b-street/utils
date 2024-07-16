use geo::{LineString, Polygon};

use crate::OffsetCurve;

pub fn buffer_linestring(
    linestring: &LineString,
    left_meters: f64,
    right_meters: f64,
) -> Option<Polygon> {
    assert!(left_meters >= 0.0);
    assert!(right_meters >= 0.0);
    let left = linestring.offset_curve(-left_meters)?;
    let right = linestring.offset_curve(right_meters)?;
    // Make a polygon by gluing these points together
    let mut pts = left.0;
    pts.reverse();
    pts.extend(right.0);
    Some(Polygon::new(LineString(pts), Vec::new()))
}
