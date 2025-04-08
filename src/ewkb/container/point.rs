use crate::ewkb::encoding::*;
use crate::ewkb::point::*;
use crate::ewkb::{EwkbPoint, EwkbRead, EwkbWrite};
use crate::{error::Error, types as postgis};
use byteorder::LittleEndian;
use byteorder::WriteBytesExt;
use std::fmt;
use std::io::{Read, Write};
use std::iter::FromIterator;
use std::slice::Iter;

macro_rules! point_container_type {
    // geometries containing points
    ($geotypetrait:ident for $geotype:ident) => {
        /// $geotypetrait
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(PartialEq, Clone, Debug)]
        pub struct $geotype<P: postgis::Point + EwkbRead> {
            pub points: Vec<P>,
            pub srid: Option<i32>,
        }

        impl<P: postgis::Point + EwkbRead> Default for $geotype<P> {
            fn default() -> Self {
                Self::new()
            }
        }

        impl<P: postgis::Point + EwkbRead> $geotype<P> {
            pub fn new() -> $geotype<P> {
                $geotype {
                    points: Vec::new(),
                    srid: None,
                }
            }
        }

        impl<P> FromIterator<P> for $geotype<P>
        where
            P: postgis::Point + EwkbRead,
        {
            #[inline]
            fn from_iter<I: IntoIterator<Item = P>>(iterable: I) -> $geotype<P> {
                let iterator = iterable.into_iter();
                let (lower, _) = iterator.size_hint();
                let mut ret = $geotype::new();
                ret.points.reserve(lower);
                for item in iterator {
                    ret.points.push(item);
                }
                ret
            }
        }

        impl<'a, P> postgis::$geotypetrait<'a> for $geotype<P>
        where
            P: 'a + postgis::Point + EwkbRead,
        {
            type ItemType = P;
            type Iter = Iter<'a, Self::ItemType>;
            fn points(&'a self) -> Self::Iter {
                self.points.iter()
            }
        }
    };
}

macro_rules! impl_read_for_point_container_type {
    (singletype $geotype:ident) => {
        impl<P> EwkbRead for $geotype<P>
        where
            P: postgis::Point + EwkbRead,
        {
            fn point_type() -> PointType {
                P::point_type()
            }
            fn read_ewkb_body<R: Read>(
                raw: &mut R,
                is_be: bool,
                type_id: u32,
                srid: Option<i32>,
            ) -> Result<Self, Error> {
                let mut points: Vec<P> = vec![];
                let size = read_u32(raw, is_be)? as usize;
                for _ in 0..size {
                    points.push(P::read_ewkb_body(raw, is_be, type_id, srid)?);
                }
                Ok($geotype::<P> {
                    points,
                    srid,
                })
            }
        }
    };
    (multitype $geotype:ident) => {
        impl<P> EwkbRead for $geotype<P>
        where
            P: postgis::Point + EwkbRead,
        {
            fn point_type() -> PointType {
                P::point_type()
            }
            fn read_ewkb_body<R: Read>(
                raw: &mut R,
                is_be: bool,
                _type_id: u32,
                srid: Option<i32>,
            ) -> Result<Self, Error> {
                let mut points: Vec<P> = vec![];
                let size = read_u32(raw, is_be)? as usize;
                for _ in 0..size {
                    points.push(P::read_ewkb(raw)?);
                }
                Ok($geotype::<P> {
                    points,
                    srid,
                })
            }
        }
    };
}

macro_rules! point_container_write {
    ($geotypetrait:ident and $asewkbtype:ident for $geotype:ident to $ewkbtype:ident with type code $typecode:expr, command $writecmd:ident) => {
        pub struct $ewkbtype<'a, P, I>
        where
            P: 'a + postgis::Point,
            I: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
        {
            pub geom: &'a dyn postgis::$geotypetrait<'a, ItemType = P, Iter = I>,
            pub srid: Option<i32>,
            pub point_type: PointType,
        }

        pub trait $asewkbtype<'a> {
            type PointType: 'a + postgis::Point;
            type Iter: Iterator<Item = &'a Self::PointType>
                + ExactSizeIterator<Item = &'a Self::PointType>;
            fn as_ewkb(&'a self) -> $ewkbtype<'a, Self::PointType, Self::Iter>;
        }

        impl<'a, T, I> fmt::Debug for $ewkbtype<'a, T, I>
        where
            T: 'a + postgis::Point,
            I: 'a + Iterator<Item = &'a T> + ExactSizeIterator<Item = &'a T>,
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, stringify!($ewkbtype))?; //TODO
                Ok(())
            }
        }

        impl<'a, T, I> EwkbWrite for $ewkbtype<'a, T, I>
        where
            T: 'a + postgis::Point,
            I: 'a + Iterator<Item = &'a T> + ExactSizeIterator<Item = &'a T>,
        {
            fn opt_srid(&self) -> Option<i32> {
                self.srid
            }

            fn type_id(&self) -> u32 {
                $typecode | Self::wkb_type_id(&self.point_type, self.srid)
            }

            fn write_ewkb_body<W: Write + ?Sized>(&self, w: &mut W) -> Result<(), Error> {
                w.write_u32::<LittleEndian>(self.geom.points().len() as u32)?;
                for geom in self.geom.points() {
                    let wkb = EwkbPoint {
                        geom,
                        srid: None,
                        point_type: self.point_type.clone(),
                    };
                    wkb.$writecmd(w)?;
                }
                Ok(())
            }
        }

        impl<'a, P> $asewkbtype<'a> for $geotype<P>
        where
            P: 'a + postgis::Point + EwkbRead,
        {
            type PointType = P;
            type Iter = Iter<'a, P>;
            fn as_ewkb(&'a self) -> $ewkbtype<'a, Self::PointType, Self::Iter> {
                $ewkbtype {
                    geom: self,
                    srid: self.srid,
                    point_type: Self::PointType::point_type(),
                }
            }
        }
    };
}

point_container_type!(LineString for LineStringT);
impl_read_for_point_container_type!(singletype LineStringT);
point_container_write!(LineString and AsEwkbLineString for LineStringT
                       to EwkbLineString with type code 0x02,
                       command write_ewkb_body);

/// OGC LineString type
pub type LineString = LineStringT<Point>;
/// OGC LineStringZ type
pub type LineStringZ = LineStringT<PointZ>;
/// OGC LineStringM type
pub type LineStringM = LineStringT<PointM>;
/// OGC LineStringZM type
pub type LineStringZM = LineStringT<PointZM>;

point_container_type!(MultiPoint for MultiPointT);
impl_read_for_point_container_type!(multitype MultiPointT);
point_container_write!(MultiPoint and AsEwkbMultiPoint for MultiPointT
                       to EwkbMultiPoint with type code 0x04,
                       command write_ewkb);

/// OGC MultiPoint type
pub type MultiPoint = MultiPointT<Point>;
/// OGC MultiPointZ type
pub type MultiPointZ = MultiPointT<PointZ>;
/// OGC MultiPointM type
pub type MultiPointM = MultiPointT<PointM>;
/// OGC MultiPointZM type
pub type MultiPointZM = MultiPointT<PointZM>;
