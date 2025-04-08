//! Read and write geometries in [OGC WKB](http://www.opengeospatial.org/standards/sfa) format.
//!
//! Support for SRID information according to [PostGIS EWKB extensions](https://git.osgeo.org/gitea/postgis/postgis/src/branch/master/doc/ZMSgeoms.txt)

mod encoding;
use crate::{error::Error, types as postgis};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use encoding::*;
use std;
use std::fmt;
use std::io::prelude::*;
use std::iter::FromIterator;
use std::slice::Iter;

// Re-export point types
pub mod point;
pub use point::*;
pub mod container;
pub use container::point::*;
mod geometry;
pub use geometry::*;

// --- Traits

pub trait EwkbRead: fmt::Debug + Sized {
    fn point_type() -> PointType;

    fn read_ewkb<R: Read>(raw: &mut R) -> Result<Self, Error> {
        let byte_order = raw.read_i8()?;
        let is_be = byte_order == 0i8;

        let type_id = read_u32(raw, is_be)?;
        let mut srid: Option<i32> = None;
        if type_id & 0x20000000 == 0x20000000 {
            srid = Some(read_i32(raw, is_be)?);
        }
        Self::read_ewkb_body(raw, is_be, type_id, srid)
    }

    #[doc(hidden)]
    fn read_ewkb_body<R: Read>(
        raw: &mut R,
        is_be: bool,
        type_id: u32,
        srid: Option<i32>,
    ) -> Result<Self, Error>;
}

pub trait EwkbWrite: fmt::Debug + Sized {
    fn opt_srid(&self) -> Option<i32> {
        None
    }

    fn wkb_type_id(point_type: &PointType, srid: Option<i32>) -> u32 {
        let mut type_ = 0;
        if srid.is_some() {
            type_ |= 0x20000000;
        }
        if *point_type == PointType::PointZ || *point_type == PointType::PointZM {
            type_ |= 0x80000000;
        }
        if *point_type == PointType::PointM || *point_type == PointType::PointZM {
            type_ |= 0x40000000;
        }
        type_
    }

    fn type_id(&self) -> u32;

    fn write_ewkb<W: Write + ?Sized>(&self, w: &mut W) -> Result<(), Error> {
        // use LE
        w.write_u8(0x01)?;
        let type_id = self.type_id();
        w.write_u32::<LittleEndian>(type_id)?;
        self.opt_srid()
            .map(|srid| w.write_i32::<LittleEndian>(srid));
        self.write_ewkb_body(w)?;
        Ok(())
    }
    #[doc(hidden)]
    fn write_ewkb_body<W: Write + ?Sized>(&self, w: &mut W) -> Result<(), Error>;

    fn to_hex_ewkb(&self) -> String {
        let mut buf: Vec<u8> = Vec::new();
        self.write_ewkb(&mut buf).unwrap();
        let hex: String = buf
            .iter()
            .fold(String::new(), |s, &b| s + &format!("{:02X}", b));
        hex
    }
}

// --- helpers

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Read(format!("error while reading: {:?}", e))
    }
}

// --- Point

fn has_z(type_id: u32) -> bool {
    type_id & 0x80000000 == 0x80000000
}
fn has_m(type_id: u32) -> bool {
    type_id & 0x40000000 == 0x40000000
}

#[test]
#[rustfmt::skip]
fn test_point_write() {
    // 'POINT (10 -20)'
    let point = Point::new(10.0, -20.0, None);
    assert_eq!(point.as_ewkb().to_hex_ewkb(), "0101000000000000000000244000000000000034C0");

    // 'POINT (10 -20 100)'
    let point = PointZ { x: 10.0, y: -20.0, z: 100.0, srid: None };
    assert_eq!(point.as_ewkb().to_hex_ewkb(), "0101000080000000000000244000000000000034C00000000000005940");

    // 'POINTM (10 -20 1)'
    let point = PointM { x: 10.0, y: -20.0, m: 1.0, srid: None };
    assert_eq!(point.as_ewkb().to_hex_ewkb(), "0101000040000000000000244000000000000034C0000000000000F03F");

    // 'POINT (10 -20 100 1)'
    let point = PointZM { x: 10.0, y: -20.0, z: 100.0, m: 1.0, srid: None };
    assert_eq!(point.as_ewkb().to_hex_ewkb(), "01010000C0000000000000244000000000000034C00000000000005940000000000000F03F");

    // 'POINT (-0 -1)'
    let point = Point::new(0.0, -1.0, None);
    assert_eq!(point.as_ewkb().to_hex_ewkb(), "01010000000000000000000000000000000000F0BF");
    // TODO: -0 in PostGIS gives 01010000000000000000000080000000000000F0BF

    // 'SRID=4326;POINT (10 -20)'
    let point = Point::new(10.0, -20.0, Some(4326));
    assert_eq!(point.as_ewkb().to_hex_ewkb(), "0101000020E6100000000000000000244000000000000034C0");
}

#[test]
#[rustfmt::skip]
fn test_line_write() {
    let p = |x, y| Point::new(x, y, None);
    // 'LINESTRING (10 -20, 0 -0.5)'
    let line = LineStringT::<Point> {srid: None, points: vec![p(10.0, -20.0), p(0., -0.5)]};
    assert_eq!(line.as_ewkb().to_hex_ewkb(), "010200000002000000000000000000244000000000000034C00000000000000000000000000000E0BF");

    // 'SRID=4326;LINESTRING (10 -20, 0 -0.5)'
    let line = LineStringT::<Point> {srid: Some(4326), points: vec![p(10.0, -20.0), p(0., -0.5)]};
    assert_eq!(line.as_ewkb().to_hex_ewkb(), "0102000020E610000002000000000000000000244000000000000034C00000000000000000000000000000E0BF");

    let p = |x, y, z| PointZ { x, y, z, srid: Some(4326) };
    // 'SRID=4326;LINESTRING (10 -20 100, 0 0.5 101)'
    let line = LineStringT::<PointZ> {srid: Some(4326), points: vec![p(10.0, -20.0, 100.0), p(0., -0.5, 101.0)]};
    assert_eq!(line.as_ewkb().to_hex_ewkb(), "01020000A0E610000002000000000000000000244000000000000034C000000000000059400000000000000000000000000000E0BF0000000000405940");
}

#[test]
#[rustfmt::skip]
fn test_polygon_write() {
    let p = |x, y| Point::new(x, y, Some(4326));
    // SELECT 'SRID=4326;POLYGON ((0 0, 2 0, 2 2, 0 2, 0 0))'::geometry
    let line = LineStringT::<Point> {srid: Some(4326), points: vec![p(0., 0.), p(2., 0.), p(2., 2.), p(0., 2.), p(0., 0.)]};
    let poly = PolygonT::<Point> {srid: Some(4326), rings: vec![line]};
    assert_eq!(poly.as_ewkb().to_hex_ewkb(), "0103000020E610000001000000050000000000000000000000000000000000000000000000000000400000000000000000000000000000004000000000000000400000000000000000000000000000004000000000000000000000000000000000");
}

#[test]
#[rustfmt::skip]
fn test_multipoint_write() {
    let p = |x, y, z| PointZ { x, y, z, srid: Some(4326) };
    // SELECT 'SRID=4326;MULTIPOINT ((10 -20 100), (0 -0.5 101))'::geometry
    let points = MultiPointT::<PointZ> {srid: Some(4326), points: vec![p(10.0, -20.0, 100.0), p(0., -0.5, 101.0)]};
    assert_eq!(points.as_ewkb().to_hex_ewkb(), "01040000A0E6100000020000000101000080000000000000244000000000000034C0000000000000594001010000800000000000000000000000000000E0BF0000000000405940");
}

#[test]
#[rustfmt::skip]
fn test_multiline_write() {
    let p = |x, y| Point::new(x, y, Some(4326));
    // SELECT 'SRID=4326;MULTILINESTRING ((10 -20, 0 -0.5), (0 0, 2 0))'::geometry
    let line1 = LineStringT::<Point> {srid: Some(4326), points: vec![p(10.0, -20.0), p(0., -0.5)]};
    let line2 = LineStringT::<Point> {srid: Some(4326), points: vec![p(0., 0.), p(2., 0.)]};
    let multiline = MultiLineStringT::<Point> {srid: Some(4326),lines: vec![line1, line2]};
    assert_eq!(multiline.as_ewkb().to_hex_ewkb(), "0105000020E610000002000000010200000002000000000000000000244000000000000034C00000000000000000000000000000E0BF0102000000020000000000000000000000000000000000000000000000000000400000000000000000");
}

#[test]
#[rustfmt::skip]
fn test_multipolygon_write() {
    let p = |x, y| Point::new(x, y, Some(4326));
    // SELECT 'SRID=4326;MULTIPOLYGON (((0 0, 2 0, 2 2, 0 2, 0 0)), ((10 10, -2 10, -2 -2, 10 -2, 10 10)))'::geometry
    let line = LineStringT::<Point> {srid: Some(4326), points: vec![p(0., 0.), p(2., 0.), p(2., 2.), p(0., 2.), p(0., 0.)]};
    let poly1 = PolygonT::<Point> {srid: Some(4326), rings: vec![line]};
    let line = LineStringT::<Point> {srid: Some(4326), points: vec![p(10., 10.), p(-2., 10.), p(-2., -2.), p(10., -2.), p(10., 10.)]};
    let poly2 = PolygonT::<Point> {srid: Some(4326), rings: vec![line]};
    let multipoly = MultiPolygonT::<Point> {srid: Some(4326), polygons: vec![poly1, poly2]};
    assert_eq!(multipoly.as_ewkb().to_hex_ewkb(), "0106000020E610000002000000010300000001000000050000000000000000000000000000000000000000000000000000400000000000000000000000000000004000000000000000400000000000000000000000000000004000000000000000000000000000000000010300000001000000050000000000000000002440000000000000244000000000000000C0000000000000244000000000000000C000000000000000C0000000000000244000000000000000C000000000000024400000000000002440");
}

#[test]
#[rustfmt::skip]
fn test_ewkb_adapters() {
    let point = Point::new(10.0, -20.0, Some(4326));
    let ewkb = EwkbPoint { geom: &point, srid: Some(4326), point_type: PointType::Point };
    assert_eq!(ewkb.to_hex_ewkb(), "0101000020E6100000000000000000244000000000000034C0");
    assert_eq!(point.as_ewkb().to_hex_ewkb(), "0101000020E6100000000000000000244000000000000034C0");
}

#[cfg(test)]
#[rustfmt::skip]
fn hex_to_vec(hexstr: &str) -> Vec<u8> {
    hexstr.as_bytes().chunks(2).map(|chars| {
        let hb = if chars[0] <= 57 { chars[0] - 48 } else { chars[0] - 55 };
        let lb = if chars[1] <= 57 { chars[1] - 48 } else { chars[1] - 55 };
        hb * 16 + lb
    }).collect::<Vec<_>>()
}

#[test]
#[rustfmt::skip]
fn test_point_read() {
    // SELECT 'POINT(10 -20)'::geometry
    let ewkb = hex_to_vec("0101000000000000000000244000000000000034C0");
    assert_eq!(ewkb, &[1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 36, 64, 0, 0, 0, 0, 0, 0, 52, 192]);
    let point = Point::read_ewkb(&mut ewkb.as_slice()).unwrap();
    assert_eq!(point.x(), 10.0);
    assert_eq!(point.y(), -20.0);
    assert_eq!(point.srid, None);

    // SELECT 'POINT(10 -20 100)'::geometry
    let ewkb = hex_to_vec("0101000080000000000000244000000000000034C00000000000005940");
    let point = PointZ::read_ewkb(&mut ewkb.as_slice()).unwrap();
    assert_eq!(point, PointZ { x: 10.0, y: -20.0, z: 100.0, srid: None });

    let point = Point::read_ewkb(&mut ewkb.as_slice()).unwrap();
    assert_eq!(point.x(), 10.0);
    assert_eq!(point.y(), -20.0);
    assert_eq!(point.srid, None);

    // SELECT 'POINTM(10 -20 1)'::geometry
    let ewkb = hex_to_vec("0101000040000000000000244000000000000034C0000000000000F03F");
    let point = PointM::read_ewkb(&mut ewkb.as_slice()).unwrap();
    assert_eq!(point, PointM { x: 10.0, y: -20.0, m: 1.0, srid: None });

    // SELECT 'POINT(10 -20 100 1)'::geometry
    let ewkb = hex_to_vec("01010000C0000000000000244000000000000034C00000000000005940000000000000F03F");
    let point = PointZM::read_ewkb(&mut ewkb.as_slice()).unwrap();
    assert_eq!(point, PointZM { x: 10.0, y: -20.0, z: 100.0, m: 1.0, srid: None });
}

#[test]
#[rustfmt::skip]
fn test_line_read() {
    let p = |x, y| Point::new(x, y, None);
    // SELECT 'LINESTRING (10 -20, 0 -0.5)'::geometry
    let ewkb = hex_to_vec("010200000002000000000000000000244000000000000034C00000000000000000000000000000E0BF");
    let line = LineStringT::<Point>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    assert_eq!(line, LineStringT::<Point> {srid: None, points: vec![p(10.0, -20.0), p(0., -0.5)]});

    let p = |x, y, z| PointZ { x, y, z, srid: Some(4326) };
    // SELECT 'SRID=4326;LINESTRING (10 -20 100, 0 -0.5 101)'::geometry
    let ewkb = hex_to_vec("01020000A0E610000002000000000000000000244000000000000034C000000000000059400000000000000000000000000000E0BF0000000000405940");
    let line = LineStringT::<PointZ>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    assert_eq!(line, LineStringT::<PointZ> {srid: Some(4326), points: vec![p(10.0, -20.0, 100.0), p(0., -0.5, 101.0)]});
}

#[test]
#[rustfmt::skip]
fn test_polygon_read() {
    let p = |x, y| Point::new(x, y, Some(4326));
    // SELECT 'SRID=4326;POLYGON ((0 0, 2 0, 2 2, 0 2, 0 0))'::geometry
    let ewkb = hex_to_vec("0103000020E610000001000000050000000000000000000000000000000000000000000000000000400000000000000000000000000000004000000000000000400000000000000000000000000000004000000000000000000000000000000000");
    let poly = PolygonT::<Point>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    let line = LineStringT::<Point> {srid: Some(4326), points: vec![p(0., 0.), p(2., 0.), p(2., 2.), p(0., 2.), p(0., 0.)]};
    assert_eq!(poly, PolygonT::<Point> {srid: Some(4326), rings: vec![line]});
}

#[test]
#[rustfmt::skip]
fn test_multipoint_read() {
    let p = |x, y, z| PointZ { x, y, z, srid: None }; // PostGIS doesn't store SRID for sub-geometries
    // SELECT 'SRID=4326;MULTIPOINT ((10 -20 100), (0 -0.5 101))'::geometry
    let ewkb = hex_to_vec("01040000A0E6100000020000000101000080000000000000244000000000000034C0000000000000594001010000800000000000000000000000000000E0BF0000000000405940");
    let points = MultiPointT::<PointZ>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    assert_eq!(points, MultiPointT::<PointZ> {srid: Some(4326), points: vec![p(10.0, -20.0, 100.0), p(0., -0.5, 101.0)]});
}

#[test]
#[rustfmt::skip]
fn test_multiline_read() {
    let p = |x, y| Point::new(x, y, None); // PostGIS doesn't store SRID for sub-geometries
    // SELECT 'SRID=4326;MULTILINESTRING ((10 -20, 0 -0.5), (0 0, 2 0))'::geometry
    let ewkb = hex_to_vec("0105000020E610000002000000010200000002000000000000000000244000000000000034C00000000000000000000000000000E0BF0102000000020000000000000000000000000000000000000000000000000000400000000000000000");
    let poly = MultiLineStringT::<Point>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    let line1 = LineStringT::<Point> {srid: None, points: vec![p(10.0, -20.0), p(0., -0.5)]};
    let line2 = LineStringT::<Point> {srid: None, points: vec![p(0., 0.), p(2., 0.)]};
    assert_eq!(poly, MultiLineStringT::<Point> {srid: Some(4326), lines: vec![line1, line2]});
}

#[test]
#[rustfmt::skip]
fn test_multipolygon_read() {
    let p = |x, y| Point::new(x, y, None); // PostGIS doesn't store SRID for sub-geometries
    // SELECT 'SRID=4326;MULTIPOLYGON (((0 0, 2 0, 2 2, 0 2, 0 0)), ((10 10, -2 10, -2 -2, 10 -2, 10 10)))'::geometry
    let ewkb = hex_to_vec("0106000020E610000002000000010300000001000000050000000000000000000000000000000000000000000000000000400000000000000000000000000000004000000000000000400000000000000000000000000000004000000000000000000000000000000000010300000001000000050000000000000000002440000000000000244000000000000000C0000000000000244000000000000000C000000000000000C0000000000000244000000000000000C000000000000024400000000000002440");
    let multipoly = MultiPolygonT::<Point>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    let line = LineStringT::<Point> {srid: None, points: vec![p(0., 0.), p(2., 0.), p(2., 2.), p(0., 2.), p(0., 0.)]};
    let poly1 = PolygonT::<Point> {srid: None, rings: vec![line]};
    let line = LineStringT::<Point> {srid: None, points: vec![p(10., 10.), p(-2., 10.), p(-2., -2.), p(10., -2.), p(10., 10.)]};
    let poly2 = PolygonT::<Point> {srid: None, rings: vec![line]};
    assert_eq!(multipoly, MultiPolygonT::<Point> {srid: Some(4326), polygons: vec![poly1, poly2]});
}

#[test]
#[rustfmt::skip]
fn test_geometrycollection_read() {
    // SELECT 'GeometryCollection(POINT (10 10),POINT (30 30),LINESTRING (15 15, 20 20))'::geometry
    let ewkb = hex_to_vec("01070000000300000001010000000000000000002440000000000000244001010000000000000000003E400000000000003E400102000000020000000000000000002E400000000000002E4000000000000034400000000000003440");
    let geom = GeometryCollectionT::<Point>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    
    // Check basic structure
    assert_eq!(geom.geometries.len(), 3);
    assert_eq!(geom.srid, None);
    
    // Check first point
    match &geom.geometries[0] {
        GeometryT::Point(pt) => {
            assert_eq!(pt.x(), 10.0);
            assert_eq!(pt.y(), 10.0);
            assert_eq!(pt.srid, None);
        },
        _ => panic!("First geometry is not a Point")
    }
    
    // Check second point
    match &geom.geometries[1] {
        GeometryT::Point(pt) => {
            assert_eq!(pt.x(), 30.0);
            assert_eq!(pt.y(), 30.0);
            assert_eq!(pt.srid, None);
        },
        _ => panic!("Second geometry is not a Point")
    }
    
    // Check linestring
    match &geom.geometries[2] {
        GeometryT::LineString(ls) => {
            assert_eq!(ls.points.len(), 2);
            assert_eq!(ls.points[0].x(), 15.0);
            assert_eq!(ls.points[0].y(), 15.0);
            assert_eq!(ls.points[1].x(), 20.0);
            assert_eq!(ls.points[1].y(), 20.0);
        },
        _ => panic!("Third geometry is not a LineString")
    }
}

#[test]
#[rustfmt::skip]
fn test_geometry_read() {
    // SELECT 'POINT(10 -20 100 1)'::geometry
    let ewkb = hex_to_vec("01010000C0000000000000244000000000000034C00000000000005940000000000000F03F");
    let geom = GeometryT::<PointZM>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    assert_eq!(format!("{:.0?}", geom), "Point(PointZM { x: 10, y: -20, z: 100, m: 1, srid: None })");
    // SELECT 'SRID=4326;LINESTRING (10 -20 100, 0 -0.5 101)'::geometry
    let ewkb = hex_to_vec("01020000A0E610000002000000000000000000244000000000000034C000000000000059400000000000000000000000000000E0BF0000000000405940");
    let geom = GeometryT::<PointZ>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    assert_eq!(format!("{:.1?}", geom), "LineString(LineStringT { points: [PointZ { x: 10.0, y: -20.0, z: 100.0, srid: Some(4326) }, PointZ { x: 0.0, y: -0.5, z: 101.0, srid: Some(4326) }], srid: Some(4326) })");
    // SELECT 'SRID=4326;POLYGON ((0 0, 2 0, 2 2, 0 2, 0 0))'::geometry
    let ewkb = hex_to_vec("0103000020E610000001000000050000000000000000000000000000000000000000000000000000400000000000000000000000000000004000000000000000400000000000000000000000000000004000000000000000000000000000000000");
    let geom = GeometryT::<Point>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    
    // Check polygon structure
    match &geom {
        GeometryT::Polygon(poly) => {
            assert_eq!(poly.srid, Some(4326));
            assert_eq!(poly.rings.len(), 1);
            
            // Check the points in the ring
            let ring = &poly.rings[0];
            assert_eq!(ring.points.len(), 5);
            
            // Create expected points
            let expected_points = [
                (0.0, 0.0),
                (2.0, 0.0),
                (2.0, 2.0),
                (0.0, 2.0),
                (0.0, 0.0)
            ];
            
            // Verify each point in the ring
            for (i, point) in ring.points.iter().enumerate() {
                assert_eq!(point.x(), expected_points[i].0);
                assert_eq!(point.y(), expected_points[i].1);
                assert_eq!(point.srid, Some(4326));
            }
        },
        _ => panic!("Geometry is not a Polygon")
    }
    // SELECT 'SRID=4326;MULTIPOINT ((10 -20 100), (0 -0.5 101))'::geometry
    let ewkb = hex_to_vec("01040000A0E6100000020000000101000080000000000000244000000000000034C0000000000000594001010000800000000000000000000000000000E0BF0000000000405940");
    let geom = GeometryT::<PointZ>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    assert_eq!(format!("{:.1?}", geom), "MultiPoint(MultiPointT { points: [PointZ { x: 10.0, y: -20.0, z: 100.0, srid: None }, PointZ { x: 0.0, y: -0.5, z: 101.0, srid: None }], srid: Some(4326) })");
    // SELECT 'SRID=4326;MULTILINESTRING ((10 -20, 0 -0.5), (0 0, 2 0))'::geometry
    let ewkb = hex_to_vec("0105000020E610000002000000010200000002000000000000000000244000000000000034C00000000000000000000000000000E0BF0102000000020000000000000000000000000000000000000000000000000000400000000000000000");
    let geom = GeometryT::<Point>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    
    // Check multilinestring structure
    match &geom {
        GeometryT::MultiLineString(mls) => {
            assert_eq!(mls.srid, Some(4326));
            assert_eq!(mls.lines.len(), 2);
            
            // First linestring
            let line1 = &mls.lines[0];
            assert_eq!(line1.points.len(), 2);
            assert_eq!(line1.points[0].x(), 10.0);
            assert_eq!(line1.points[0].y(), -20.0);
            assert_eq!(line1.points[1].x(), 0.0);
            assert_eq!(line1.points[1].y(), -0.5);
            
            // Second linestring
            let line2 = &mls.lines[1];
            assert_eq!(line2.points.len(), 2);
            assert_eq!(line2.points[0].x(), 0.0);
            assert_eq!(line2.points[0].y(), 0.0);
            assert_eq!(line2.points[1].x(), 2.0);
            assert_eq!(line2.points[1].y(), 0.0);
        },
        _ => panic!("Geometry is not a MultiLineString")
    };
    // SELECT 'SRID=4326;MULTIPOLYGON (((0 0, 2 0, 2 2, 0 2, 0 0)), ((10 10, -2 10, -2 -2, 10 -2, 10 10)))'::geometry
    let ewkb = hex_to_vec("0106000020E610000002000000010300000001000000050000000000000000000000000000000000000000000000000000400000000000000000000000000000004000000000000000400000000000000000000000000000004000000000000000000000000000000000010300000001000000050000000000000000002440000000000000244000000000000000C0000000000000244000000000000000C000000000000000C0000000000000244000000000000000C000000000000024400000000000002440");
    let geom = GeometryT::<Point>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    
    // Check multipolygon structure
    match &geom {
        GeometryT::MultiPolygon(mpoly) => {
            assert_eq!(mpoly.srid, Some(4326));
            assert_eq!(mpoly.polygons.len(), 2);
            
            // First polygon
            let poly1 = &mpoly.polygons[0];
            assert_eq!(poly1.rings.len(), 1);
            let ring1 = &poly1.rings[0];
            assert_eq!(ring1.points.len(), 5);
            
            // Check coordinates of first polygon
            let points1 = [
                (0.0, 0.0),
                (2.0, 0.0),
                (2.0, 2.0),
                (0.0, 2.0),
                (0.0, 0.0)
            ];
            
            for (i, pt) in ring1.points.iter().enumerate() {
                assert_eq!(pt.x(), points1[i].0);
                assert_eq!(pt.y(), points1[i].1);
            }
            
            // Second polygon
            let poly2 = &mpoly.polygons[1];
            assert_eq!(poly2.rings.len(), 1);
            let ring2 = &poly2.rings[0];
            assert_eq!(ring2.points.len(), 5);
            
            // Check coordinates of second polygon
            let points2 = [
                (10.0, 10.0),
                (-2.0, 10.0),
                (-2.0, -2.0),
                (10.0, -2.0),
                (10.0, 10.0)
            ];
            
            for (i, pt) in ring2.points.iter().enumerate() {
                assert_eq!(pt.x(), points2[i].0);
                assert_eq!(pt.y(), points2[i].1);
            }
        },
        _ => panic!("Geometry is not a MultiPolygon")
    };
    // SELECT 'GeometryCollection(POINT (10 10),POINT (30 30),LINESTRING (15 15, 20 20))'::geometry
    let ewkb = hex_to_vec("01070000000300000001010000000000000000002440000000000000244001010000000000000000003E400000000000003E400102000000020000000000000000002E400000000000002E4000000000000034400000000000003440");
    let geom = GeometryT::<Point>::read_ewkb(&mut ewkb.as_slice()).unwrap();
    
    // Check geometry collection structure
    match &geom {
        GeometryT::GeometryCollection(gc) => {
            assert_eq!(gc.srid, None);
            assert_eq!(gc.geometries.len(), 3);
            
            // First point
            match &gc.geometries[0] {
                GeometryT::Point(pt) => {
                    assert_eq!(pt.x(), 10.0);
                    assert_eq!(pt.y(), 10.0);
                },
                _ => panic!("First geometry is not a Point")
            }
            
            // Second point
            match &gc.geometries[1] {
                GeometryT::Point(pt) => {
                    assert_eq!(pt.x(), 30.0);
                    assert_eq!(pt.y(), 30.0);
                },
                _ => panic!("Second geometry is not a Point")
            }
            
            // LineString
            match &gc.geometries[2] {
                GeometryT::LineString(ls) => {
                    assert_eq!(ls.points.len(), 2);
                    assert_eq!(ls.points[0].x(), 15.0);
                    assert_eq!(ls.points[0].y(), 15.0);
                    assert_eq!(ls.points[1].x(), 20.0);
                    assert_eq!(ls.points[1].y(), 20.0);
                },
                _ => panic!("Third geometry is not a LineString")
            }
        },
        _ => panic!("Geometry is not a GeometryCollection")
    };
}

#[test]
#[rustfmt::skip]
fn test_read_error() {
    // SELECT 'LINESTRING (10 -20, 0 -0.5)'::geometry
    let ewkb = hex_to_vec("010200000002000000000000000000244000000000000034C00000000000000000000000000000E0BF");
    let poly = PolygonT::<Point>::read_ewkb(&mut ewkb.as_slice());
    assert!(poly.is_err()); // UnexpectedEof "failed to fill whole buffer"
}

#[test]
#[rustfmt::skip]
fn test_iterators() {
    // Iterator traits:
    use crate::types::LineString;

    let p = |x, y| Point::new(x, y, None);
    let line = self::LineStringT::<Point> {srid: Some(4326), points: vec![p(10.0, -20.0), p(0., -0.5)]};
    let last_point = line.points().last().unwrap();
    assert_eq!(last_point.x(), 0.);
    assert_eq!(last_point.y(), -0.5);
    assert_eq!(last_point.srid, None);
}

#[cfg(all(test, feature = "serde"))]
mod serde_tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_serde_point() {
        let point = Point::new(10.0, 20.0, Some(4326));

        let serialized = serde_json::to_string(&point).unwrap();
        let deserialized: Point = serde_json::from_str(&serialized).unwrap();

        assert_eq!(point, deserialized);
    }

    #[test]
    fn test_serde_point_z() {
        let point = PointZ {
            x: 10.0,
            y: 20.0,
            z: 30.0,
            srid: Some(4326),
        };

        let serialized = serde_json::to_string(&point).unwrap();
        let deserialized: PointZ = serde_json::from_str(&serialized).unwrap();

        assert_eq!(point, deserialized);
    }

    #[test]
    fn test_serde_geometry_t() {
        let point = Point::new(10.0, 20.0, Some(4326));
        let geometry = GeometryT::Point(point);

        let serialized = serde_json::to_string(&geometry).unwrap();
        let deserialized: GeometryT<Point> = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            GeometryT::Point(p) => assert_eq!(p, point),
            _ => panic!("Deserialized to wrong variant"),
        }
    }
}
