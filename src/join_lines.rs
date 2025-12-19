use std::collections::HashMap;
use std::hash::Hash;

use geo::{Coord, LineString};

/// A linestring with a list of IDs in order, and an arbitrary key
pub struct KeyedLineString<ID, K> {
    pub linestring: LineString,
    // True if forwards, false if backwards
    pub ids: Vec<(ID, bool)>,
    pub key: K,
}

impl<ID, K: Copy + Eq + Hash> KeyedLineString<ID, K> {
    fn first_pt(&self) -> HashedPoint<K> {
        HashedPoint::new(*self.linestring.0.first().unwrap(), self.key)
    }

    fn last_pt(&self) -> HashedPoint<K> {
        HashedPoint::new(*self.linestring.0.last().unwrap(), self.key)
    }

    // TODO Assumes not a loop
    fn other_endpt(&self, pt: HashedPoint<K>) -> HashedPoint<K> {
        if self.first_pt() == pt {
            self.last_pt()
        } else {
            self.first_pt()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct HashedPoint<K: Hash + Eq>(isize, isize, K);

impl<K: Hash + Eq> HashedPoint<K> {
    fn new(pt: Coord, key: K) -> Self {
        // cm precision
        Self((pt.x * 100.0) as isize, (pt.y * 100.0) as isize, key)
    }
}

/// Takes a network of linestrings. Finds every case of exactly two linestrings meeting at a point,
/// and merges them together. Only linestrings with a matching key are considered. The linestrings
/// can track an underlying road or edge ID, and the result will retain that detailed semantic
/// path. Two pieces of a loop are not collapsed.
pub fn collapse_degree_2<ID, K: Copy + Eq + Hash>(
    input_lines: Vec<KeyedLineString<ID, K>>,
) -> Vec<KeyedLineString<ID, K>> {
    // Assign each input an ID that doesn't change
    let mut lines: HashMap<usize, KeyedLineString<ID, K>> =
        input_lines.into_iter().enumerate().collect();
    let mut id_counter = lines.len();

    // How many lines connect to each point?
    let mut point_to_line: HashMap<HashedPoint<K>, Vec<usize>> = HashMap::new();
    for (id, line) in &lines {
        point_to_line
            .entry(line.first_pt())
            .or_insert_with(Vec::new)
            .push(*id);
        point_to_line
            .entry(line.last_pt())
            .or_insert_with(Vec::new)
            .push(*id);
    }

    // Find all degree 2 cases
    let degree_two: Vec<HashedPoint<K>> = point_to_line
        .iter()
        .filter(|(_, list)| list.len() == 2)
        .map(|(pt, _)| *pt)
        .collect();

    // Fix them
    for pt in degree_two {
        let pair = point_to_line.remove(&pt).unwrap();
        let (idx1, idx2) = (pair[0], pair[1]);

        // Check if the two lines form a loop, and if so, skip
        if is_loop(&lines[&idx1], &lines[&idx2]) {
            point_to_line.insert(pt, pair);
            continue;
        }

        let line1 = lines.remove(&idx1).unwrap();
        let line2 = lines.remove(&idx2).unwrap();
        let other_endpt1 = line1.other_endpt(pt);
        let other_endpt2 = line2.other_endpt(pt);

        let joined = join_lines(line1, line2);
        lines.insert(id_counter, joined);

        // Fix up point_to_line
        replace(
            point_to_line.get_mut(&other_endpt1).unwrap(),
            idx1,
            id_counter,
        );
        replace(
            point_to_line.get_mut(&other_endpt2).unwrap(),
            idx2,
            id_counter,
        );

        id_counter += 1;
    }

    lines.into_values().collect()
}

/// Like collapse_degree_2, but only combines pairs of input that form a loop.
pub fn collapse_loops<ID, K: Copy + Eq + Hash>(
    input_lines: Vec<KeyedLineString<ID, K>>,
) -> Vec<KeyedLineString<ID, K>> {
    // Assign each input an ID that doesn't change
    let mut lines: HashMap<usize, KeyedLineString<ID, K>> =
        input_lines.into_iter().enumerate().collect();
    let mut id_counter = lines.len();

    // How many lines connect to each point?
    let mut point_to_line: HashMap<HashedPoint<K>, Vec<usize>> = HashMap::new();
    for (id, line) in &lines {
        point_to_line
            .entry(line.first_pt())
            .or_insert_with(Vec::new)
            .push(*id);
        point_to_line
            .entry(line.last_pt())
            .or_insert_with(Vec::new)
            .push(*id);
    }

    // Find all pairs of loops
    let loop_pairs: Vec<(usize, usize)> = point_to_line
        .iter()
        .filter(|(_, list)| list.len() == 2 && is_loop(&lines[&list[0]], &lines[&list[1]]))
        .map(|(_, list)| (list[0], list[1]))
        .collect();

    // Fix them
    for (idx1, idx2) in loop_pairs {
        // Each pair will show up twice and only be fixed the first time
        if lines.contains_key(&idx1) && lines.contains_key(&idx2) {
            let line1 = lines.remove(&idx1).unwrap();
            let line2 = lines.remove(&idx2).unwrap();
            let joined = join_lines(line1, line2);
            lines.insert(id_counter, joined);
            id_counter += 1;

            // Unlike collapse_degree_2, don't fix up point_to_line or anything else. After merging
            // a loop, no further work is ever necessary.
        }
    }

    lines.into_values().collect()
}

fn is_loop<ID, K: Copy + Eq + Hash>(
    line1: &KeyedLineString<ID, K>,
    line2: &KeyedLineString<ID, K>,
) -> bool {
    let (pt1, pt2) = (line1.first_pt(), line1.last_pt());
    let (pt3, pt4) = (line2.first_pt(), line2.last_pt());
    (pt1 == pt3 || pt1 == pt4) && (pt2 == pt3 || pt2 == pt4)
}

fn join_lines<ID, K: Copy + Eq + Hash>(
    mut line1: KeyedLineString<ID, K>,
    mut line2: KeyedLineString<ID, K>,
) -> KeyedLineString<ID, K> {
    let (pt1, pt2) = (line1.first_pt(), line1.last_pt());
    let (pt3, pt4) = (line2.first_pt(), line2.last_pt());

    if pt1 == pt3 {
        line1.linestring.0.reverse();
        line1.linestring.0.pop();
        line1.linestring.0.extend(line2.linestring.0);

        line1.ids.reverse();
        flip_direction(&mut line1.ids);
        line1.ids.extend(line2.ids);
    } else if pt1 == pt4 {
        line2.linestring.0.pop();
        line2.linestring.0.extend(line1.linestring.0);
        line1.linestring.0 = line2.linestring.0;

        line2.ids.extend(line1.ids);
        line1.ids = line2.ids;
    } else if pt2 == pt3 {
        line1.linestring.0.pop();
        line1.linestring.0.extend(line2.linestring.0);

        line1.ids.extend(line2.ids);
    } else if pt2 == pt4 {
        line2.linestring.0.reverse();
        line1.linestring.0.pop();
        line1.linestring.0.extend(line2.linestring.0);

        line2.ids.reverse();
        flip_direction(&mut line2.ids);
        line1.ids.extend(line2.ids);
    } else {
        unreachable!()
    }

    line1
}

fn flip_direction<ID>(ids: &mut Vec<(ID, bool)>) {
    for pair in ids {
        pair.1 = !pair.1;
    }
}

fn replace(indices: &mut Vec<usize>, old: usize, new: usize) {
    for x in indices {
        if *x == old {
            *x = new;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::line_string;

    #[test]
    fn test_easy() {
        let input = vec![
            KeyedLineString {
                linestring: line_string![(x: 0., y: 0.), (x: 0., y: 5.)],
                ids: vec![("r1", true)],
                key: (),
            },
            KeyedLineString {
                linestring: line_string![(x: 0., y: 5.), (x: 0., y: 10.)],
                ids: vec![("r2", true)],
                key: (),
            },
        ];
        let output = collapse_degree_2(input);
        assert_eq!(1, output.len());
        // TODO Could require more traits
        assert_eq!(
            output[0].linestring,
            line_string![(x: 0., y: 0.), (x: 0., y: 5.), (x: 0., y: 10.)]
        );
        assert_eq!(output[0].ids, vec![("r1", true), ("r2", true)]);
    }

    #[test]
    fn test_loop() {
        let make_input = || {
            vec![
                KeyedLineString {
                    linestring: line_string![(x: 0., y: 0.), (x: 0., y: 5.)],
                    ids: vec![("r1", true)],
                    key: (),
                },
                KeyedLineString {
                    linestring: line_string![(x: 0., y: 5.), (x: 0., y: 0.)],
                    ids: vec![("r2", true)],
                    key: (),
                },
            ]
        };

        {
            let output = collapse_degree_2(make_input());
            // There should be no change
            assert_eq!(2, output.len());
        }

        {
            let output = collapse_loops(make_input());
            assert_eq!(1, output.len());
            // Depending on hash ordering, two outputs are possible
            let case1 = output[0].linestring
                == line_string![(x: 0., y: 0.), (x: 0., y: 5.), (x: 0., y: 0.)];
            let case2 = output[0].linestring
                == line_string![(x: 0., y: 5.), (x: 0., y: 0.), (x: 0., y: 5.)];

            if case1 {
                assert_eq!(output[0].ids, vec![("r1", true), ("r2", true)]);
            } else if case2 {
                assert_eq!(output[0].ids, vec![("r2", true), ("r1", true)]);
            } else {
                panic!("loop didn't merge: {:?}", output[0].linestring);
            }
        }
    }
}
