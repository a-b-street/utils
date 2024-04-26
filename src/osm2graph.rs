use std::collections::HashMap;

use anyhow::Result;
use geo::{ConvexHull, Coord, Geometry, GeometryCollection, LineString, Point, Polygon};
use osm_reader::{Element, NodeID, WayID};

use crate::{Mercator, Tags};

/// Don't use this as a final structure, just an intermediate helper for splitting OSM ways into
/// edges
pub struct Graph {
    pub edges: Vec<Edge>,
    /// Nodes in the graph sense, not OSM, though they happen to correspond to one OSM node
    // TODO Rename, but don't be confusing
    pub intersections: Vec<Intersection>,
    // All geometry is stored in world-space
    pub mercator: Mercator,
    pub boundary_polygon: Polygon,
}

#[derive(Clone, Copy)]
pub struct EdgeID(pub usize);
//#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[derive(Clone, Copy)]
pub struct IntersectionID(pub usize);

pub struct Edge {
    pub id: EdgeID,
    pub src: IntersectionID,
    pub dst: IntersectionID,

    pub osm_way: osm_reader::WayID,
    pub osm_node1: osm_reader::NodeID,
    pub osm_node2: osm_reader::NodeID,

    pub linestring: LineString,
}

pub struct Intersection {
    pub id: IntersectionID,
    pub edges: Vec<EdgeID>,

    pub osm_node: osm_reader::NodeID,

    pub point: Point,
}

/// A scraped OSM way
pub struct Way {
    pub id: WayID,
    pub node_ids: Vec<NodeID>,
    pub tags: Tags,
}

impl Graph {
    pub fn new<KeepEdge: Fn(&Tags) -> bool>(
        input_bytes: &[u8],
        keep_edge: KeepEdge,
    ) -> Result<Self> {
        let mut node_mapping = HashMap::new();
        let mut highways = Vec::new();
        osm_reader::parse(input_bytes, |elem| match elem {
            Element::Node { id, lon, lat, .. } => {
                let pt = Coord { x: lon, y: lat };
                node_mapping.insert(id, pt);
            }
            Element::Way {
                id, node_ids, tags, ..
            } => {
                let tags: Tags = tags.into();
                if keep_edge(&tags) {
                    highways.push(Way { id, node_ids, tags });
                }
            }
            Element::Relation { .. } => {}
            Element::Bounds { .. } => {}
        })?;

        let (mut edges, mut intersections) = split_edges(&node_mapping, highways);

        // TODO expensive
        let mut collection: GeometryCollection = edges
            .iter()
            .map(|e| Geometry::LineString(e.linestring.clone()))
            .chain(
                intersections
                    .iter()
                    .map(|i| Geometry::Point(i.point.clone())),
            )
            .collect::<Vec<_>>()
            .into();
        let mercator = Mercator::from(collection.clone()).unwrap();
        for e in &mut edges {
            mercator.to_mercator_in_place(&mut e.linestring);
        }
        for i in &mut intersections {
            mercator.to_mercator_in_place(&mut i.point);
        }

        mercator.to_mercator_in_place(&mut collection);
        let boundary_polygon = collection.convex_hull();

        Ok(Self {
            edges,
            intersections,
            mercator,
            boundary_polygon,
        })
    }
}

fn split_edges(
    node_mapping: &HashMap<NodeID, Coord>,
    ways: Vec<Way>,
) -> (Vec<Edge>, Vec<Intersection>) {
    // Count how many ways reference each node
    let mut node_counter: HashMap<NodeID, usize> = HashMap::new();
    for way in &ways {
        for node in &way.node_ids {
            *node_counter.entry(*node).or_insert(0) += 1;
        }
    }

    // Split each way into edges
    let mut node_to_intersection: HashMap<NodeID, IntersectionID> = HashMap::new();
    let mut intersections = Vec::new();
    let mut edges = Vec::new();
    for way in ways {
        let mut node1 = way.node_ids[0];
        let mut pts = Vec::new();

        let num_nodes = way.node_ids.len();
        for (idx, node) in way.node_ids.into_iter().enumerate() {
            pts.push(node_mapping[&node]);
            // Edges start/end at intersections between two ways. The endpoints of the way also
            // count as intersections.
            let is_endpoint =
                idx == 0 || idx == num_nodes - 1 || *node_counter.get(&node).unwrap() > 1;
            if is_endpoint && pts.len() > 1 {
                let edge_id = EdgeID(edges.len());

                let mut i_ids = Vec::new();
                for (n, point) in [(node1, pts[0]), (node, *pts.last().unwrap())] {
                    let intersection = if let Some(i) = node_to_intersection.get(&n) {
                        &mut intersections[i.0]
                    } else {
                        let i = IntersectionID(intersections.len());
                        intersections.push(Intersection {
                            id: i,
                            osm_node: n,
                            point: Point(point),
                            edges: Vec::new(),
                        });
                        node_to_intersection.insert(n, i);
                        &mut intersections[i.0]
                    };

                    intersection.edges.push(edge_id);
                    i_ids.push(intersection.id);
                }

                edges.push(Edge {
                    id: edge_id,
                    src: i_ids[0],
                    dst: i_ids[1],
                    osm_way: way.id,
                    osm_node1: node1,
                    osm_node2: node,
                    linestring: LineString::new(std::mem::take(&mut pts)),
                });

                // Start the next edge
                node1 = node;
                pts.push(node_mapping[&node]);
            }
        }
    }

    (edges, intersections)
}
