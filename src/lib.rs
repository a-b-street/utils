mod debugger;
mod grid;
mod join_lines;
mod line_split;
mod mercator;
#[cfg(feature = "serde")]
mod node_map;
mod offset_curve;
pub mod osm2graph;
mod priority_queue;
mod split_polygon;
mod step_along_line;
mod tags;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use self::debugger::Debugger;
pub use self::grid::Grid;
pub use self::join_lines::{collapse_degree_2, KeyedLineString};
pub use self::line_split::{LineSplit, LineSplitResult, LineSplitTwiceResult};
pub use self::mercator::Mercator;
#[cfg(feature = "serde")]
pub use self::node_map::{deserialize_nodemap, NodeMap};
pub use self::offset_curve::OffsetCurve;
pub use self::priority_queue::PriorityQueueItem;
pub use self::split_polygon::split_polygon;
pub use self::step_along_line::step_along_line;
pub use self::tags::Tags;
#[cfg(target_arch = "wasm32")]
pub use self::wasm::download_string;

use geo::{BoundingRect, Point, Rect};
use rstar::AABB;

pub fn aabb<G: BoundingRect<f64, Output = Option<Rect<f64>>>>(geom: &G) -> AABB<Point> {
    let bbox: Rect = geom.bounding_rect().unwrap().into();
    AABB::from_corners(
        Point::new(bbox.min().x, bbox.min().y),
        Point::new(bbox.max().x, bbox.max().y),
    )
}

/// Expand an AABB by some amount on all sides
pub fn buffer_aabb(aabb: AABB<Point>, buffer_meters: f64) -> AABB<Point> {
    AABB::from_corners(
        Point::new(
            aabb.lower().x() - buffer_meters,
            aabb.lower().y() - buffer_meters,
        ),
        Point::new(
            aabb.upper().x() + buffer_meters,
            aabb.upper().y() + buffer_meters,
        ),
    )
}
