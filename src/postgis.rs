//
// Copyright (c) ShuYu Wang <andelf@gmail.com>, Feather Workshop and Pirmin
// Kalberer. All rights reserved.
//

use crate::{
	ewkb::{
		self, AsEwkbGeometry, AsEwkbGeometryCollection, AsEwkbLineString, AsEwkbMultiLineString,
		AsEwkbMultiPoint, AsEwkbMultiPolygon, AsEwkbPoint, AsEwkbPolygon, EwkbRead, EwkbWrite,
	},
	twkb::{self, TwkbGeom},
	types::{LineString, Point, Polygon},
};
use bytes::{BufMut, BytesMut};
use postgres_types::{FromSql, IsNull, ToSql, Type, accepts, to_sql_checked};
use std::{error::Error, io::Cursor};

macro_rules! accepts_geography {
	() => {
		fn accepts(ty: &Type) -> bool {
			match ty.name() {
				"geography" | "geometry" => true,
				_ => false,
			}
		}
	};
}

impl ToSql for ewkb::EwkbPoint<'_> {
	accepts_geography!();

	to_sql_checked!();

	fn to_sql(&self, _: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
		self.write_ewkb(&mut out.writer())?;
		Ok(IsNull::No)
	}
}

macro_rules! impl_sql_for_point_type {
	($ptype:ident) => {
		impl<'a> FromSql<'a> for ewkb::$ptype {
			accepts_geography!();

			fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
				let mut rdr = Cursor::new(raw);
				ewkb::$ptype::read_ewkb(&mut rdr)
					.map_err(|_| format!("cannot convert {} to {}", ty, stringify!($ptype)).into())
			}
		}

		impl ToSql for ewkb::$ptype {
			to_sql_checked!();

			accepts_geography!();

			fn to_sql(
				&self,
				_: &Type,
				out: &mut BytesMut,
			) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
				self.as_ewkb().write_ewkb(&mut out.writer())?;
				Ok(IsNull::No)
			}
		}
	};
}

impl_sql_for_point_type!(Point);
impl_sql_for_point_type!(PointZ);
impl_sql_for_point_type!(PointM);
impl_sql_for_point_type!(PointZM);

macro_rules! impl_sql_for_geom_type {
	($geotype:ident) => {
		impl<'a, T> FromSql<'a> for ewkb::$geotype<T>
		where
			T: 'a + Point + EwkbRead,
		{
			accepts_geography!();

			fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
				let mut rdr = Cursor::new(raw);
				ewkb::$geotype::<T>::read_ewkb(&mut rdr).map_err(|_| {
					format!("cannot convert {} to {}", ty, stringify!($geotype)).into()
				})
			}
		}

		impl<'a, T> ToSql for ewkb::$geotype<T>
		where
			T: 'a + Point + EwkbRead,
		{
			to_sql_checked!();

			accepts_geography!();

			fn to_sql(
				&self,
				_: &Type,
				out: &mut BytesMut,
			) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
				self.as_ewkb().write_ewkb(&mut out.writer())?;
				Ok(IsNull::No)
			}
		}
	};
}

impl_sql_for_geom_type!(LineStringT);
impl_sql_for_geom_type!(PolygonT);
impl_sql_for_geom_type!(MultiPointT);
impl_sql_for_geom_type!(MultiLineStringT);
impl_sql_for_geom_type!(MultiPolygonT);

macro_rules! impl_sql_for_ewkb_type {
	($ewkbtype:ident contains points) => {
		impl<'a, T, I> ToSql for ewkb::$ewkbtype<'a, T, I>
		where
			T: 'a + Point,
			I: 'a + Iterator<Item = &'a T> + ExactSizeIterator<Item = &'a T>,
		{
			to_sql_checked!();

			accepts_geography!();

			fn to_sql(
				&self,
				_: &Type,
				out: &mut BytesMut,
			) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
				self.write_ewkb(&mut out.writer())?;
				Ok(IsNull::No)
			}
		}
	};
	($ewkbtype:ident contains $itemtypetrait:ident) => {
		impl<'a, P, I, T, J> ToSql for ewkb::$ewkbtype<'a, P, I, T, J>
		where
			P: 'a + Point,
			I: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
			T: 'a + $itemtypetrait<'a, ItemType = P, Iter = I>,
			J: 'a + Iterator<Item = &'a T> + ExactSizeIterator<Item = &'a T>,
		{
			to_sql_checked!();

			accepts_geography!();

			fn to_sql(
				&self,
				_: &Type,
				out: &mut BytesMut,
			) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
				self.write_ewkb(&mut out.writer())?;
				Ok(IsNull::No)
			}
		}
	};
	(multipoly $ewkbtype:ident contains $itemtypetrait:ident) => {
		impl<'a, P, I, L, K, T, J> ToSql for ewkb::$ewkbtype<'a, P, I, L, K, T, J>
		where
			P: 'a + Point,
			I: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
			L: 'a + LineString<'a, ItemType = P, Iter = I>,
			K: 'a + Iterator<Item = &'a L> + ExactSizeIterator<Item = &'a L>,
			T: 'a + $itemtypetrait<'a, ItemType = L, Iter = K>,
			J: 'a + Iterator<Item = &'a T> + ExactSizeIterator<Item = &'a T>,
		{
			to_sql_checked!();

			accepts_geography!();

			fn to_sql(
				&self,
				_: &Type,
				out: &mut BytesMut,
			) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
				self.write_ewkb(&mut out.writer())?;
				Ok(IsNull::No)
			}
		}
	};
}

impl_sql_for_ewkb_type!(EwkbLineString contains points);
impl_sql_for_ewkb_type!(EwkbPolygon contains LineString);
impl_sql_for_ewkb_type!(EwkbMultiPoint contains points);
impl_sql_for_ewkb_type!(EwkbMultiLineString contains LineString);
impl_sql_for_ewkb_type!(multipoly EwkbMultiPolygon contains Polygon);

impl<P> FromSql<'_> for ewkb::GeometryT<P>
where
	P: Point + EwkbRead,
{
	accepts_geography!();

	fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
		let mut rdr = Cursor::new(raw);
		ewkb::GeometryT::<P>::read_ewkb(&mut rdr)
			.map_err(|_| format!("cannot convert {} to {}", ty, stringify!(P)).into())
	}
}

// NOTE: Implement once per point type because AsEwkbPoint<'a> doesn't live long
// enough for ToSql
macro_rules! impl_geometry_to_sql {
	($ptype:path) => {
		impl ToSql for ewkb::GeometryT<$ptype> {
			to_sql_checked!();

			accepts_geography!();

			fn to_sql(
				&self,
				_: &Type,
				out: &mut BytesMut,
			) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
				self.as_ewkb().write_ewkb(&mut out.writer())?;
				Ok(IsNull::No)
			}
		}
	};
}

impl_geometry_to_sql!(ewkb::Point);
impl_geometry_to_sql!(ewkb::PointZ);
impl_geometry_to_sql!(ewkb::PointM);
impl_geometry_to_sql!(ewkb::PointZM);

impl<P> FromSql<'_> for ewkb::GeometryCollectionT<P>
where
	P: Point + EwkbRead,
{
	accepts_geography!();

	fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
		let mut rdr = Cursor::new(raw);
		ewkb::GeometryCollectionT::<P>::read_ewkb(&mut rdr)
			.map_err(|_| format!("cannot convert {} to {}", ty, stringify!(P)).into())
	}
}

impl<P> ToSql for ewkb::GeometryCollectionT<P>
where
	P: Point + EwkbRead,
{
	to_sql_checked!();

	accepts_geography!();

	fn to_sql(&self, _: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
		self.as_ewkb().write_ewkb(&mut out.writer())?;
		Ok(IsNull::No)
	}
}

// --- TWKB ---

impl FromSql<'_> for twkb::Point {
	accepts!(BYTEA);

	fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
		let mut rdr = Cursor::new(raw);
		twkb::Point::read_twkb(&mut rdr)
			.map_err(|_| format!("cannot convert {} to Point", ty).into())
	}
}

impl FromSql<'_> for twkb::LineString {
	accepts!(BYTEA);

	fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
		let mut rdr = Cursor::new(raw);
		twkb::LineString::read_twkb(&mut rdr)
			.map_err(|_| format!("cannot convert {} to LineString", ty).into())
	}
}

impl FromSql<'_> for twkb::Polygon {
	accepts!(BYTEA);

	fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
		let mut rdr = Cursor::new(raw);
		twkb::Polygon::read_twkb(&mut rdr)
			.map_err(|_| format!("cannot convert {} to Polygon", ty).into())
	}
}

impl FromSql<'_> for twkb::MultiPoint {
	accepts!(BYTEA);

	fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
		let mut rdr = Cursor::new(raw);
		twkb::MultiPoint::read_twkb(&mut rdr)
			.map_err(|_| format!("cannot convert {} to MultiPoint", ty).into())
	}
}

impl FromSql<'_> for twkb::MultiLineString {
	accepts!(BYTEA);

	fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
		let mut rdr = Cursor::new(raw);
		twkb::MultiLineString::read_twkb(&mut rdr)
			.map_err(|_| format!("cannot convert {} to MultiLineString", ty).into())
	}
}

impl FromSql<'_> for twkb::MultiPolygon {
	accepts!(BYTEA);

	fn from_sql(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
		let mut rdr = Cursor::new(raw);
		twkb::MultiPolygon::read_twkb(&mut rdr)
			.map_err(|_| format!("cannot convert {} to MultiPolygon", ty).into())
	}
}

#[cfg(test)]
mod tests {
	use crate::{
		ewkb::{self, AsEwkbLineString, AsEwkbPoint},
		twkb, types as postgis,
	};
	use postgres::{Client, NoTls};
	use std::env;

	macro_rules! or_panic {
		($e:expr) => {
			match $e {
				Ok(ok) => ok,
				Err(err) => panic!("{:#?}", err),
			}
		};
	}

	fn connect() -> Client {
		match env::var("DBCONN") {
			Result::Ok(val) => Client::connect(&val as &str, NoTls),
			Result::Err(err) => panic!("{:#?}", err),
		}
		.unwrap()
	}

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_insert_point() {
        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(Point))", &[]));

        // 'POINT (10 -20)'
        let point = ewkb::Point::new(10.0, -20.0, None);
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&point]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('POINT(10 -20)') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
        or_panic!(client.execute("TRUNCATE geomtests", &[]));

        // With SRID
        let point = ewkb::Point::new(10.0, -20.0, Some(4326));
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&point]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('SRID=4326;POINT(10 -20)') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
        or_panic!(client.execute("TRUNCATE geomtests", &[]));

        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(Point, 4326))", &[]));

        let point = ewkb::Point::new(10.0, -20.0, Some(4326));
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&point]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('SRID=4326;POINT(10 -20)') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
        or_panic!(client.execute("TRUNCATE geomtests", &[]));

        // Missing SRID
        let point = ewkb::Point::new(10.0, -20.0, None);
        let result = client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&point]);
        assert!(format!("{}", result.err().unwrap()).starts_with("db error"));
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_insert_line() {
        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(LineString))", &[]));

        let p = |x, y| ewkb::Point::new(x, y, None);
        // 'LINESTRING (10 -20, -0 -0.5)'
        let line = ewkb::LineString {srid: None, points: vec![p(10.0, -20.0), p(0., -0.5)]};
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&line]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('LINESTRING(10 -20, 0 -0.5)') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
        or_panic!(client.execute("TRUNCATE geomtests", &[]));

        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(LineString, 4326))", &[]));

        // 'SRID=4326;LINESTRING (10 -20, -0 -0.5)'
        let line = ewkb::LineString {srid: Some(4326), points: vec![p(10.0, -20.0), p(0., -0.5)]};
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&line]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('SRID=4326;LINESTRING(10 -20, 0 -0.5)') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
        or_panic!(client.execute("TRUNCATE geomtests", &[]));

        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(LineStringZ, 4326))", &[]));

        let p = |x, y, z| ewkb::PointZ { x, y, z, srid: Some(4326) };
        // 'SRID=4326;LINESTRING (10 -20 100, -0 -0.5 101)'
        let line = ewkb::LineStringZ {srid: Some(4326), points: vec![p(10.0, -20.0, 100.0), p(0., -0.5, 101.0)]};
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&line]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('SRID=4326;LINESTRING (10 -20 100, 0 -0.5 101)') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
        or_panic!(client.execute("TRUNCATE geomtests", &[]));
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_insert_polygon() {
        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(Polygon))", &[]));
        let p = |x, y| ewkb::Point::new(x, y, Some(4326));
        // SELECT 'SRID=4326;POLYGON ((0 0, 2 0, 2 2, 0 2, 0 0))'::geometry
        let line = ewkb::LineString {srid: Some(4326), points: vec![p(0., 0.), p(2., 0.), p(2., 2.), p(0., 2.), p(0., 0.)]};
        let poly = ewkb::Polygon {srid: Some(4326), rings: vec![line]};
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&poly]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('SRID=4326;POLYGON ((0 0, 2 0, 2 2, 0 2, 0 0))') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_insert_multipoint() {
        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(MultiPointZ))", &[]));
        let p = |x, y, z| ewkb::PointZ { x, y, z, srid: Some(4326) };
        // SELECT 'SRID=4326;MULTIPOINT ((10 -20 100), (0 -0.5 101))'::geometry
        let points = ewkb::MultiPointZ {srid: Some(4326), points: vec![p(10.0, -20.0, 100.0), p(0., -0.5, 101.0)]};
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&points]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('SRID=4326;MULTIPOINT ((10 -20 100), (0 -0.5 101))') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_insert_multiline() {
        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(MultiLineString))", &[]));
        let p = |x, y| ewkb::Point::new(x, y, Some(4326));
        // SELECT 'SRID=4326;MULTILINESTRING ((10 -20, 0 -0.5), (0 0, 2 0))'::geometry
        let line1 = ewkb::LineString {srid: Some(4326), points: vec![p(10.0, -20.0), p(0., -0.5)]};
        let line2 = ewkb::LineString {srid: Some(4326), points: vec![p(0., 0.), p(2., 0.)]};
        let multiline = ewkb::MultiLineString {srid: Some(4326),lines: vec![line1, line2]};
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&multiline]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('SRID=4326;MULTILINESTRING ((10 -20, 0 -0.5), (0 0, 2 0))') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_insert_multipolygon() {
        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(MultiPolygon))", &[]));
        let p = |x, y| ewkb::Point::new(x, y, Some(4326));
        // SELECT 'SRID=4326;MULTIPOLYGON (((0 0, 2 0, 2 2, 0 2, 0 0)), ((10 10, -2 10, -2 -2, 10 -2, 10 10)))'::geometry
        let line = ewkb::LineString {srid: Some(4326), points: vec![p(0., 0.), p(2., 0.), p(2., 2.), p(0., 2.), p(0., 0.)]};
        let poly1 = ewkb::Polygon {srid: Some(4326), rings: vec![line]};
        let line = ewkb::LineString {srid: Some(4326), points: vec![p(10., 10.), p(-2., 10.), p(-2., -2.), p(10., -2.), p(10., 10.)]};
        let poly2 = ewkb::Polygon {srid: Some(4326), rings: vec![line]};
        let multipoly = ewkb::MultiPolygon {srid: Some(4326), polygons: vec![poly1, poly2]};
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&multipoly]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('SRID=4326;MULTIPOLYGON (((0 0, 2 0, 2 2, 0 2, 0 0)), ((10 10, -2 10, -2 -2, 10 -2, 10 10)))') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_insert_geometry() {
        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry)", &[]));
        let p = |x, y| ewkb::Point::new(x, y, Some(4326));
        // SELECT 'SRID=4326;MULTIPOLYGON (((0 0, 2 0, 2 2, 0 2, 0 0)), ((10 10, -2 10, -2 -2, 10 -2, 10 10)))'::geometry
        let multipoly = {
            let line = ewkb::LineString {srid: Some(4326), points: vec![p(0., 0.), p(2., 0.), p(2., 2.), p(0., 2.), p(0., 0.)]};
            let poly1 = ewkb::Polygon {srid: Some(4326), rings: vec![line]};
            let line = ewkb::LineString {srid: Some(4326), points: vec![p(10., 10.), p(-2., 10.), p(-2., -2.), p(10., -2.), p(10., 10.)]};
            let poly2 = ewkb::Polygon {srid: Some(4326), rings: vec![line]};
            ewkb::MultiPolygon {srid: Some(4326), polygons: vec![poly1, poly2]}
        };
        let geometry = ewkb::GeometryT::MultiPolygon(multipoly);
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&geometry]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('SRID=4326;MULTIPOLYGON (((0 0, 2 0, 2 2, 0 2, 0 0)), ((10 10, -2 10, -2 -2, 10 -2, 10 10)))') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_insert_geometrycollection() {
        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(GeometryCollection))", &[]));
        let p = |x, y| ewkb::Point::new(x, y, Some(4326));
        // SELECT 'SRID=4326;LINESTRING (10 -20, -0 -0.5)'
        let line = ewkb::LineString {srid: Some(4326), points: vec![p(10.0, -20.0), p(0., -0.5)]};
        // SELECT 'SRID=4326;MULTIPOLYGON (((0 0, 2 0, 2 2, 0 2, 0 0)), ((10 10, -2 10, -2 -2, 10 -2, 10 10)))'::geometry
        let multipoly = {
            let line = ewkb::LineString {srid: Some(4326), points: vec![p(0., 0.), p(2., 0.), p(2., 2.), p(0., 2.), p(0., 0.)]};
            let poly1 = ewkb::Polygon {srid: Some(4326), rings: vec![line]};
            let line = ewkb::LineString {srid: Some(4326), points: vec![p(10., 10.), p(-2., 10.), p(-2., -2.), p(10., -2.), p(10., 10.)]};
            let poly2 = ewkb::Polygon {srid: Some(4326), rings: vec![line]};
            ewkb::MultiPolygon {srid: Some(4326), polygons: vec![poly1, poly2]}
        };
        // SELECT 'SRID=4326;GEOMETRYCOLLECTION (LINESTRING (10 -20,0 -0.5), MULTIPOLYGON (((0 0,2 0,2 2,0 2,0 0)),((10 10,-2 10,-2 -2,10 -2,10 10))))'::geometry
        let collection = ewkb::GeometryCollection{
            srid: Some(4326),
            geometries: vec![
                ewkb::GeometryT::LineString(line),
                ewkb::GeometryT::MultiPolygon(multipoly),
            ],
        };
        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&collection]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('SRID=4326;GEOMETRYCOLLECTION (LINESTRING (10 -20,0 -0.5), MULTIPOLYGON (((0 0,2 0,2 2,0 2,0 0)),((10 10,-2 10,-2 -2,10 -2,10 10))))') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_select_point() {
        let mut client = connect();
        let result = or_panic!(client.query("SELECT ('POINT(10 -20)')::geometry", &[]));
        let point = result.iter().map(|r| r.get::<_, ewkb::Point>(0)).last().unwrap();
        let expected = ewkb::Point::new(10.0, -20.0, None);
        assert_eq!(point.x(), expected.x());
        assert_eq!(point.y(), expected.y());
        assert_eq!(point.srid, expected.srid);

        let result = or_panic!(client.query("SELECT 'SRID=4326;POINT(10 -20)'::geometry", &[]));
        let point = result.iter().map(|r| r.get::<_, ewkb::Point>(0)).last().unwrap();
        let expected = ewkb::Point::new(10.0, -20.0, Some(4326));
        assert_eq!(point.x(), expected.x());
        assert_eq!(point.y(), expected.y());
        assert_eq!(point.srid, expected.srid);

        let result = or_panic!(client.query("SELECT 'SRID=4326;POINT(10 -20 99)'::geometry", &[]));
        let point = result.iter().map(|r| r.get::<_, ewkb::PointZ>(0)).last().unwrap();
        assert_eq!(point, ewkb::PointZ { x: 10.0, y: -20.0, z: 99.0, srid: Some(4326) });

        let result = or_panic!(client.query("SELECT 'POINT EMPTY'::geometry", &[]));
        let point = result.iter().map(|r| r.get::<_, ewkb::Point>(0)).last().unwrap();
        assert_eq!(&format!("{:?}", point), "Point { x: NaN, y: NaN, srid: None }");

        let result = or_panic!(client.query("SELECT NULL::geometry(Point)", &[]));
        let point = result.iter().map(|r| r.try_get::<_, ewkb::Point>(0)).last().unwrap();
        assert_eq!(&format!("{:?}", point), "Err(Error { kind: FromSql(0), cause: Some(WasNull) })");
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_select_line() {
        let mut client = connect();
        let p = |x, y| ewkb::Point::new(x, y, None);
        let result = or_panic!(client.query("SELECT ('LINESTRING (10 -20, -0 -0.5)')::geometry", &[]));
        let line = result.iter().map(|r| r.get::<_, ewkb::LineString>(0)).last().unwrap();
        assert_eq!(line, ewkb::LineString {srid: None, points: vec![p(10.0, -20.0), p(0., -0.5)]});

        let p = |x, y| ewkb::Point::new(x, y, Some(4326));
        let result = or_panic!(client.query("SELECT ('SRID=4326;LINESTRING (10 -20, -0 -0.5)')::geometry", &[]));
        let line = result.iter().map(|r| r.get::<_, ewkb::LineString>(0)).last().unwrap();
        assert_eq!(line, ewkb::LineString {srid: Some(4326), points: vec![p(10.0, -20.0), p(0., -0.5)]});

        let p = |x, y| ewkb::Point::new(x, y, Some(4326));
        let result = or_panic!(client.query("SELECT ('SRID=4326;LINESTRINGZ (10 -20 1, -0 -0.5 1)')::geometry", &[]));
        let line = result.iter().map(|r| r.get::<_, ewkb::LineString>(0)).last().unwrap();
        assert_eq!(line, ewkb::LineString {srid: Some(4326), points: vec![p(10.0, -20.0), p(0., -0.5)]});

        let result = or_panic!(client.query("SELECT 'LINESTRING EMPTY'::geometry", &[]));
        let line = result.iter().map(|r| r.get::<_, ewkb::LineString>(0)).last().unwrap();
        assert_eq!(&format!("{:?}", line), "LineStringT { points: [], srid: None }");
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_select_polygon() {
        let mut client = connect();
        let result = or_panic!(client.query("SELECT 'SRID=4326;POLYGON ((0 0, 2 0, 2 2, 0 2, 0 0))'::geometry", &[]));
        let poly = result.iter().map(|r| r.get::<_, ewkb::Polygon>(0)).last().unwrap();
        assert_eq!(format!("{:.0?}", poly), "PolygonT { rings: [LineStringT { points: [Point { x: 0, y: 0, srid: Some(4326) }, Point { x: 2, y: 0, srid: Some(4326) }, Point { x: 2, y: 2, srid: Some(4326) }, Point { x: 0, y: 2, srid: Some(4326) }, Point { x: 0, y: 0, srid: Some(4326) }], srid: Some(4326) }], srid: Some(4326) }");
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_select_multipoint() {
        let mut client = connect();
        let result = or_panic!(client.query("SELECT 'SRID=4326;MULTIPOINT ((10 -20 100), (0 -0.5 101))'::geometry", &[]));
        let points = result.iter().map(|r| r.get::<_, ewkb::MultiPointZ>(0)).last().unwrap();
        assert_eq!(format!("{:.1?}", points), "MultiPointT { points: [PointZ { x: 10.0, y: -20.0, z: 100.0, srid: None }, PointZ { x: 0.0, y: -0.5, z: 101.0, srid: None }], srid: Some(4326) }");
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_select_multiline() {
        let mut client = connect();
        let result = or_panic!(client.query("SELECT 'SRID=4326;MULTILINESTRING ((10 -20, 0 -0.5), (0 0, 2 0))'::geometry", &[]));
        let multiline = result.iter().map(|r| r.get::<_, ewkb::MultiLineString>(0)).last().unwrap();
        assert_eq!(format!("{:.1?}", multiline), "MultiLineStringT { lines: [LineStringT { points: [Point { x: 10.0, y: -20.0, srid: None }, Point { x: 0.0, y: -0.5, srid: None }], srid: None }, LineStringT { points: [Point { x: 0.0, y: 0.0, srid: None }, Point { x: 2.0, y: 0.0, srid: None }], srid: None }], srid: Some(4326) }");
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_select_multipolygon() {
        let mut client = connect();
        let result = or_panic!(client.query("SELECT 'SRID=4326;MULTIPOLYGON (((0 0, 2 0, 2 2, 0 2, 0 0)), ((10 10, -2 10, -2 -2, 10 -2, 10 10)))'::geometry", &[]));
        let multipoly = result.iter().map(|r| r.get::<_, ewkb::MultiPolygon>(0)).last().unwrap();
        assert_eq!(format!("{:.0?}", multipoly), "MultiPolygonT { polygons: [PolygonT { rings: [LineStringT { points: [Point { x: 0, y: 0, srid: None }, Point { x: 2, y: 0, srid: None }, Point { x: 2, y: 2, srid: None }, Point { x: 0, y: 2, srid: None }, Point { x: 0, y: 0, srid: None }], srid: None }], srid: None }, PolygonT { rings: [LineStringT { points: [Point { x: 10, y: 10, srid: None }, Point { x: -2, y: 10, srid: None }, Point { x: -2, y: -2, srid: None }, Point { x: 10, y: -2, srid: None }, Point { x: 10, y: 10, srid: None }], srid: None }], srid: None }], srid: Some(4326) }");
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_select_geometrycollection() {
        let mut client = connect();
        let result = or_panic!(client.query("SELECT 'GeometryCollection(POINT (10 10),POINT (30 30),LINESTRING (15 15, 20 20))'::geometry", &[]));
        let geom = result.iter().map(|r| r.get::<_, ewkb::GeometryCollection>(0)).last().unwrap();
        assert_eq!(format!("{:.0?}", geom), "GeometryCollectionT { geometries: [Point(Point { x: 10, y: 10, srid: None }), Point(Point { x: 30, y: 30, srid: None }), LineString(LineStringT { points: [Point { x: 15, y: 15, srid: None }, Point { x: 20, y: 20, srid: None }], srid: None })], srid: None }");
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_select_geometry() {
        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry)", &[]));
        or_panic!(client.execute("INSERT INTO geomtests VALUES('SRID=4326;POINT(10 -20 99)'::geometry)", &[]));
        let result = or_panic!(client.query("SELECT geom FROM geomtests", &[]));
        let geom = result.iter().map(|r| r.get::<_, ewkb::GeometryZ>(0)).last().unwrap();
        assert_eq!(format!("{:.0?}", geom), "Point(PointZ { x: 10, y: -20, z: 99, srid: Some(4326) })");
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_select_type_error() {
        let mut client = connect();
        let result = or_panic!(client.query("SELECT ('LINESTRING (10 -20, -0 -0.5)')::geometry", &[]));
        let poly = result.iter().map(|r| r.try_get::<_, ewkb::Polygon>(0)).last().unwrap();
        assert_eq!(format!("{:?}", poly), "Err(Error { kind: FromSql(0), cause: Some(\"cannot convert geometry to PolygonT\") })");
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_twkb() {
        let mut client = connect();
        let result = or_panic!(client.query("SELECT ST_AsTWKB('POINT(10 -20)'::geometry)", &[]));
        let point = result.iter().map(|r| r.get::<_, twkb::Point>(0)).last().unwrap();
        assert_eq!(point, twkb::Point {x: 10.0, y: -20.0});

        let result = or_panic!(client.query("SELECT ST_AsTWKB('SRID=4326;POINT(10 -20)'::geometry)", &[]));
        let point = result.iter().map(|r| r.get::<_, twkb::Point>(0)).last().unwrap();
        assert_eq!(point, twkb::Point {x: 10.0, y: -20.0});

        let result = or_panic!(client.query("SELECT ST_AsTWKB('POINT EMPTY'::geometry)", &[]));
        let point = result.iter().map(|r| r.get::<_, twkb::Point>(0)).last().unwrap();
        assert_eq!(&format!("{:?}", point), "Point { x: NaN, y: NaN }");
        let point = &point as &dyn postgis::Point;
        assert!(point.x().is_nan());

        let result = or_panic!(client.query("SELECT ST_AsTWKB(NULL::geometry(Point))", &[]));
        let point = result.iter().map(|r| r.try_get::<_, twkb::Point>(0)).last().unwrap();
        assert_eq!(&format!("{:?}", point), "Err(Error { kind: FromSql(0), cause: Some(WasNull) })");

        let result = or_panic!(client.query("SELECT ST_AsTWKB('LINESTRING (10 -20, -0 -0.5)'::geometry, 1)", &[]));
        let line = result.iter().map(|r| r.get::<_, twkb::LineString>(0)).last().unwrap();
        assert_eq!(&format!("{:.1?}", line), "LineString { points: [Point { x: 10.0, y: -20.0 }, Point { x: 0.0, y: -0.5 }] }");
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    fn test_twkb_insert() {
        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(Point))", &[]));

        let result = or_panic!(client.query("SELECT ST_AsTWKB('POINT(10 -20)'::geometry)", &[]));
        let point = result.iter().map(|r| r.get::<_, twkb::Point>(0)).last().unwrap();

        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&point.as_ewkb()]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('POINT(10 -20)') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
        or_panic!(client.execute("TRUNCATE geomtests", &[]));

        let mut client = connect();
        or_panic!(client.execute("CREATE TEMPORARY TABLE geomtests (geom geometry(LineString))", &[]));

        let result = or_panic!(client.query("SELECT ST_AsTWKB('LINESTRING (10 -20, 0 -0.5)'::geometry, 1)", &[]));
        let line = result.iter().map(|r| r.get::<_, twkb::LineString>(0)).last().unwrap();

        or_panic!(client.execute("INSERT INTO geomtests (geom) VALUES ($1)", &[&line.as_ewkb()]));
        let result = or_panic!(client.query("SELECT geom=ST_GeomFromEWKT('LINESTRING (10 -20, 0 -0.5)') FROM geomtests", &[]));
        assert!(result.iter().map(|r| r.get::<_, bool>(0)).last().unwrap());
        or_panic!(client.execute("TRUNCATE geomtests", &[]));
    }

	#[test]
    #[ignore]
    #[rustfmt::skip]
    #[allow(unused_imports,unused_variables)]
    fn test_examples() {
        use postgres::{Client, NoTls};
        //use postgis::ewkb;
        //use postgis::LineString;

        fn main() {
            //
            use crate::{
                ewkb,
                types::LineString,
                twkb
            };
            let mut client = connect();
            or_panic!(client.execute("CREATE TEMPORARY TABLE busline (route geometry(LineString))", &[]));
            or_panic!(client.execute("CREATE TEMPORARY TABLE stops (stop geometry(Point))", &[]));
            or_panic!(client.execute("INSERT INTO busline (route) VALUES ('LINESTRING(10 -20, -0 -0.5)'::geometry)", &[]));
            //

            // conn ....
            for row in &client.query("SELECT * FROM busline", &[]).unwrap() {
                let route: ewkb::LineString = row.get("route");
                let last_stop = route.points().last().unwrap();
                let _ = client.execute("INSERT INTO stops (stop) VALUES ($1)", &[&last_stop]);
            }

            for row in &client.query("SELECT * FROM busline", &[]).unwrap() {
                let route = row.try_get::<_, Option<ewkb::LineString>>("route");
                match route {
                    Ok(Some(geom)) => { println!("{:?}", geom) }
                    Ok(None) => { /* Handle NULL value */ }
                    Err(err) => { println!("Error: {}", err) }
                }
            }

        //use postgis::twkb;

            for row in &client.query("SELECT ST_AsTWKB(route) FROM busline", &[]).unwrap() {
                let route: twkb::LineString = row.get(0);
                let last_stop = route.points().last().unwrap();
                let _ = client.execute("INSERT INTO stops (stop) VALUES ($1)", &[&last_stop.as_ewkb()]);
            }
        }

        main();
    }
}
