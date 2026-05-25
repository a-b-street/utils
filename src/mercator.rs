use anyhow::Result;
use geo::{BoundingRect, Coord, Haversine, Length, LineString, MapCoords, MapCoordsInPlace, Rect};
use geojson::{Feature, Geometry, GeometryValue};
use proj4rs::Proj;
use serde::{Deserialize, Serialize};

/// Projects WGS84 points onto a Euclidean plane, using a Mercator projection. The top-left is (0,
/// 0) and grows to the right and down (screen-drawing order, not Cartesian), with units of meters.
/// The accuracy of this weakens for larger areas.
///
/// If `new_from_proj` then this is a total misnomer -- the Euclidean plane is determined by the
/// CRS. Use for large scales where Mercator is too lossy. Serializing/deserializing will not work.
// TODO Upstream or consider https://github.com/georust/geo/issues/1165
#[derive(Clone, Serialize, Deserialize)]
pub struct Mercator {
    pub wgs84_bounds: Rect,
    pub width: f64,
    pub height: f64,

    /// (WGS84, the custom one)
    #[serde(skip_serializing, skip_deserializing)]
    proj: Option<(Proj, Proj)>,
}

impl Mercator {
    // TODO The API is kind of annoying, or wasteful. Do builder style.
    /// Create a boundary covering some geometry
    pub fn from<T: BoundingRect<f64>>(geometry: T) -> Option<Self> {
        let wgs84_bounds = geometry.bounding_rect().into()?;
        let width = Haversine.length(&LineString::from(vec![
            (wgs84_bounds.min().x, wgs84_bounds.min().y),
            (wgs84_bounds.max().x, wgs84_bounds.min().y),
        ]));
        let height = Haversine.length(&LineString::from(vec![
            (wgs84_bounds.min().x, wgs84_bounds.min().y),
            (wgs84_bounds.min().x, wgs84_bounds.max().y),
        ]));
        Some(Self {
            wgs84_bounds,
            width,
            height,

            proj: None,
        })
    }

    pub fn new_from_proj(crs: &str) -> Result<Self> {
        let wgs84 = Proj::from_user_string("WGS84")?;
        let custom = Proj::from_user_string(crs)?;

        Ok(Self {
            wgs84_bounds: Rect::new(Coord { x: 0., y: 0. }, Coord { x: 0., y: 0. }),
            width: 0.,
            height: 0.,

            proj: Some((wgs84, custom)),
        })
    }

    pub fn pt_to_mercator(&self, pt: Coord) -> Coord {
        if let Some((wgs84, custom)) = &self.proj {
            let mut pt = (pt.x.to_radians(), pt.y.to_radians());
            proj4rs::transform::transform(wgs84, custom, &mut pt).unwrap();
            return Coord { x: pt.0, y: pt.1 };
        }

        let x = self.width * (pt.x - self.wgs84_bounds.min().x) / self.wgs84_bounds.width();
        // Invert y, so that the northernmost latitude is 0
        let y = self.height
            - self.height * (pt.y - self.wgs84_bounds.min().y) / self.wgs84_bounds.height();
        Coord { x, y }
    }

    pub fn pt_to_wgs84(&self, pt: Coord) -> Coord {
        if let Some((wgs84, custom)) = &self.proj {
            let mut pt = (pt.x, pt.y);
            proj4rs::transform::transform(custom, wgs84, &mut pt).unwrap();
            return Coord {
                x: trim_lon_lat(pt.0.to_degrees()),
                y: trim_lon_lat(pt.1.to_degrees()),
            };
        }

        let x = trim_lon_lat(
            self.wgs84_bounds.min().x + (pt.x / self.width * self.wgs84_bounds.width()),
        );
        let y = trim_lon_lat(
            self.wgs84_bounds.min().y
                + (self.wgs84_bounds.height() * (self.height - pt.y) / self.height),
        );
        Coord { x, y }
    }

    pub fn to_mercator<G: MapCoords<f64, f64, Output = G>>(&self, geom: &G) -> G {
        geom.map_coords(|pt| self.pt_to_mercator(pt))
    }

    pub fn to_wgs84<G: MapCoords<f64, f64, Output = G>>(&self, geom: &G) -> G {
        geom.map_coords(|pt| self.pt_to_wgs84(pt))
    }

    pub fn to_wgs84_gj<G: MapCoords<f64, f64, Output = G>>(&self, geom: &G) -> Feature
    where
        GeometryValue: for<'a> From<&'a G>,
    {
        Feature::from(Geometry::from(GeometryValue::from(&self.to_wgs84(geom))))
    }

    pub fn to_mercator_in_place<G: MapCoordsInPlace<f64>>(&self, geom: &mut G) {
        geom.map_coords_in_place(|pt| self.pt_to_mercator(pt));
    }

    pub fn to_wgs84_in_place<G: MapCoordsInPlace<f64>>(&self, geom: &mut G) {
        geom.map_coords_in_place(|pt| self.pt_to_wgs84(pt));
    }
}

// Per https://datatracker.ietf.org/doc/html/rfc7946#section-11.2, 6 decimal places (10cm) is
// plenty of precision
fn trim_lon_lat(x: f64) -> f64 {
    (x * 10e6).round() / 10e6
}
