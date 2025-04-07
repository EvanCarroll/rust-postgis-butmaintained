use crate::{error::Error, types as postgis};
use geo_types::geometry::Point as _Point;
use std::io::prelude::*;

use super::{has_m, has_z, read_f64, AsEwkbPoint, EwkbPoint, EwkbRead};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum PointType {
    Point,
    PointZ,
    PointM,
    PointZM,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub struct Point {
    #[cfg_attr(feature = "serde", derive(serde::flatten))]
    pub point: _Point,
    pub srid: Option<i32>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub struct PointZ {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub srid: Option<i32>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub struct PointM {
    pub x: f64,
    pub y: f64,
    pub m: f64,
    pub srid: Option<i32>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Clone, Copy, Debug, Default)]
pub struct PointZM {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub m: f64,
    pub srid: Option<i32>,
}

impl Point {
    pub fn new(x: f64, y: f64, srid: Option<i32>) -> Self {
        Self {
            point: _Point::new(x, y),
            srid,
        }
    }
    pub fn new_from_opt_vals(
        x: f64,
        y: f64,
        _z: Option<f64>,
        _m: Option<f64>,
        srid: Option<i32>,
    ) -> Self {
        Self::new(x, y, srid)
    }

    pub fn x(&self) -> f64 {
        self.point.x()
    }

    pub fn y(&self) -> f64 {
        self.point.y()
    }
}

impl From<(f64, f64)> for Point {
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y, None)
    }
}

impl postgis::Point for Point {
    fn x(&self) -> f64 {
        self.point.x()
    }
    fn y(&self) -> f64 {
        self.point.y()
    }
}

impl PointZ {
    pub fn new(x: f64, y: f64, z: f64, srid: Option<i32>) -> Self {
        Self { x, y, z, srid }
    }
    pub fn new_from_opt_vals(
        x: f64,
        y: f64,
        z: Option<f64>,
        _m: Option<f64>,
        srid: Option<i32>,
    ) -> Self {
        Self::new(x, y, z.unwrap_or(0.0), srid)
    }
}

impl From<(f64, f64, f64)> for PointZ {
    fn from((x, y, z): (f64, f64, f64)) -> Self {
        Self::new(x, y, z, None)
    }
}

impl postgis::Point for PointZ {
    fn x(&self) -> f64 {
        self.x
    }
    fn y(&self) -> f64 {
        self.y
    }
    fn opt_z(&self) -> Option<f64> {
        Some(self.z)
    }
}

impl PointM {
    pub fn new(x: f64, y: f64, m: f64, srid: Option<i32>) -> Self {
        Self { x, y, m, srid }
    }
    pub fn new_from_opt_vals(
        x: f64,
        y: f64,
        _z: Option<f64>,
        m: Option<f64>,
        srid: Option<i32>,
    ) -> Self {
        Self::new(x, y, m.unwrap_or(0.0), srid)
    }
}

impl From<(f64, f64, f64)> for PointM {
    fn from((x, y, m): (f64, f64, f64)) -> Self {
        Self::new(x, y, m, None)
    }
}

impl postgis::Point for PointM {
    fn x(&self) -> f64 {
        self.x
    }
    fn y(&self) -> f64 {
        self.y
    }
    fn opt_m(&self) -> Option<f64> {
        Some(self.m)
    }
}

impl PointZM {
    pub fn new(x: f64, y: f64, z: f64, m: f64, srid: Option<i32>) -> Self {
        Self { x, y, z, m, srid }
    }
    pub fn new_from_opt_vals(
        x: f64,
        y: f64,
        z: Option<f64>,
        m: Option<f64>,
        srid: Option<i32>,
    ) -> Self {
        Self::new(x, y, z.unwrap_or(0.0), m.unwrap_or(0.0), srid)
    }
}

impl From<(f64, f64, f64, f64)> for PointZM {
    fn from((x, y, z, m): (f64, f64, f64, f64)) -> Self {
        Self::new(x, y, z, m, None)
    }
}

impl postgis::Point for PointZM {
    fn x(&self) -> f64 {
        self.x
    }
    fn y(&self) -> f64 {
        self.y
    }
    fn opt_z(&self) -> Option<f64> {
        Some(self.z)
    }
    fn opt_m(&self) -> Option<f64> {
        Some(self.m)
    }
}

macro_rules! impl_point_read_traits {
    ($ptype:ident) => {
        impl EwkbRead for $ptype {
            fn point_type() -> PointType {
                PointType::$ptype
            }
            fn read_ewkb_body<R: Read>(
                raw: &mut R,
                is_be: bool,
                type_id: u32,
                srid: Option<i32>,
            ) -> Result<Self, Error> {
                let x = read_f64(raw, is_be)?;
                let y = read_f64(raw, is_be)?;
                let z = if has_z(type_id) {
                    Some(read_f64(raw, is_be)?)
                } else {
                    None
                };
                let m = if has_m(type_id) {
                    Some(read_f64(raw, is_be)?)
                } else {
                    None
                };
                Ok(Self::new_from_opt_vals(x, y, z, m, srid))
            }
        }

        impl<'a> AsEwkbPoint<'a> for $ptype {
            fn as_ewkb(&'a self) -> EwkbPoint<'a> {
                EwkbPoint {
                    geom: self,
                    srid: self.srid,
                    point_type: PointType::$ptype,
                }
            }
        }
    };
}

impl_point_read_traits!(Point);
impl_point_read_traits!(PointZ);
impl_point_read_traits!(PointM);
impl_point_read_traits!(PointZM);
