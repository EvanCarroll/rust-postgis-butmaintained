use crate::ewkb::*;

macro_rules! geometry_container_type {
    // geometries containing lines and polygons
    ($geotypetrait:ident for $geotype:ident contains $itemtype:ident named $itemname:ident) => {
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(PartialEq, Clone, Debug)]
        pub struct $geotype<P: postgis::Point + EwkbRead> {
            pub $itemname: Vec<$itemtype<P>>,
            pub srid: Option<i32>,
        }

        impl<P> $geotype<P>
        where
            P: postgis::Point + EwkbRead,
        {
            pub fn new() -> $geotype<P> {
                $geotype {
                    $itemname: Vec::new(),
                    srid: None,
                }
            }
        }

        impl<P> FromIterator<$itemtype<P>> for $geotype<P>
        where
            P: postgis::Point + EwkbRead,
        {
            #[inline]
            fn from_iter<I: IntoIterator<Item = $itemtype<P>>>(iterable: I) -> $geotype<P> {
                let iterator = iterable.into_iter();
                let (lower, _) = iterator.size_hint();
                let mut ret = $geotype::new();
                ret.$itemname.reserve(lower);
                for item in iterator {
                    ret.$itemname.push(item);
                }
                ret
            }
        }

        impl<'a, P> postgis::$geotypetrait<'a> for $geotype<P>
        where
            P: 'a + postgis::Point + EwkbRead,
        {
            type ItemType = $itemtype<P>;
            type Iter = Iter<'a, Self::ItemType>;
            fn $itemname(&'a self) -> Self::Iter {
                self.$itemname.iter()
            }
        }
    };
}

macro_rules! impl_read_for_geometry_container_type {
    (singletype $geotype:ident contains $itemtype:ident named $itemname:ident) => {
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
                let mut $itemname: Vec<$itemtype<P>> = vec![];
                let size = read_u32(raw, is_be)? as usize;
                for _ in 0..size {
                    $itemname.push($itemtype::read_ewkb_body(raw, is_be, type_id, srid)?);
                }
                Ok($geotype::<P> {
                    $itemname: $itemname,
                    srid: srid,
                })
            }
        }
    };
    (multitype $geotype:ident contains $itemtype:ident named $itemname:ident) => {
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
                let mut $itemname: Vec<$itemtype<P>> = vec![];
                let size = read_u32(raw, is_be)? as usize;
                for _ in 0..size {
                    $itemname.push($itemtype::read_ewkb(raw)?);
                }
                Ok($geotype::<P> {
                    $itemname: $itemname,
                    srid: srid,
                })
            }
        }
    };
}

macro_rules! geometry_container_write {
    ($geotypetrait:ident and $asewkbtype:ident for $geotype:ident to $ewkbtype:ident with type code $typecode:expr, contains $ewkbitemtype:ident, $itemtype:ident as $itemtypetrait:ident named $itemname:ident, command $writecmd:ident) => {
        pub struct $ewkbtype<'a, P, I, T, J>
        where
            P: 'a + postgis::Point,
            I: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
            T: 'a + postgis::$itemtypetrait<'a, ItemType = P, Iter = I>,
            J: 'a + Iterator<Item = &'a T> + ExactSizeIterator<Item = &'a T>,
        {
            pub geom: &'a dyn postgis::$geotypetrait<'a, ItemType = T, Iter = J>,
            pub srid: Option<i32>,
            pub point_type: PointType,
        }

        pub trait $asewkbtype<'a> {
            type PointType: 'a + postgis::Point;
            type PointIter: Iterator<Item = &'a Self::PointType>
                + ExactSizeIterator<Item = &'a Self::PointType>;
            type ItemType: 'a
                + postgis::$itemtypetrait<'a, ItemType = Self::PointType, Iter = Self::PointIter>;
            type Iter: Iterator<Item = &'a Self::ItemType>
                + ExactSizeIterator<Item = &'a Self::ItemType>;
            fn as_ewkb(
                &'a self,
            ) -> $ewkbtype<'a, Self::PointType, Self::PointIter, Self::ItemType, Self::Iter>;
        }

        impl<'a, P, I, T, J> fmt::Debug for $ewkbtype<'a, P, I, T, J>
        where
            P: 'a + postgis::Point,
            I: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
            T: 'a + postgis::$itemtypetrait<'a, ItemType = P, Iter = I>,
            J: 'a + Iterator<Item = &'a T> + ExactSizeIterator<Item = &'a T>,
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, stringify!($ewkbtype))?; //TODO
                Ok(())
            }
        }

        impl<'a, P, I, T, J> EwkbWrite for $ewkbtype<'a, P, I, T, J>
        where
            P: 'a + postgis::Point,
            I: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
            T: 'a + postgis::$itemtypetrait<'a, ItemType = P, Iter = I>,
            J: 'a + Iterator<Item = &'a T> + ExactSizeIterator<Item = &'a T>,
        {
            fn opt_srid(&self) -> Option<i32> {
                self.srid
            }

            fn type_id(&self) -> u32 {
                $typecode | Self::wkb_type_id(&self.point_type, self.srid)
            }

            fn write_ewkb_body<W: Write + ?Sized>(&self, w: &mut W) -> Result<(), Error> {
                w.write_u32::<LittleEndian>(self.geom.$itemname().len() as u32)?;
                for geom in self.geom.$itemname() {
                    let wkb = $ewkbitemtype {
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
            type PointIter = Iter<'a, P>;
            type ItemType = $itemtype<P>;
            type Iter = Iter<'a, Self::ItemType>;
            fn as_ewkb(
                &'a self,
            ) -> $ewkbtype<'a, Self::PointType, Self::PointIter, Self::ItemType, Self::Iter> {
                $ewkbtype {
                    geom: self,
                    srid: self.srid,
                    point_type: Self::PointType::point_type(),
                }
            }
        }
    };
    (multipoly $geotypetrait:ident and $asewkbtype:ident for $geotype:ident to $ewkbtype:ident with type code $typecode:expr, contains $ewkbitemtype:ident, $itemtype:ident as $itemtypetrait:ident named $itemname:ident, command $writecmd:ident) => {
        pub struct $ewkbtype<'a, P, I, L, K, T, J>
        where
            P: 'a + postgis::Point,
            I: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
            L: 'a + postgis::LineString<'a, ItemType = P, Iter = I>,
            K: 'a + Iterator<Item = &'a L> + ExactSizeIterator<Item = &'a L>,
            T: 'a + postgis::$itemtypetrait<'a, ItemType = L, Iter = K>,
            J: 'a + Iterator<Item = &'a T> + ExactSizeIterator<Item = &'a T>,
        {
            pub geom: &'a dyn postgis::$geotypetrait<'a, ItemType = T, Iter = J>,
            pub srid: Option<i32>,
            pub point_type: PointType,
        }

        pub trait $asewkbtype<'a> {
            type PointType: 'a + postgis::Point;
            type PointIter: Iterator<Item = &'a Self::PointType>
                + ExactSizeIterator<Item = &'a Self::PointType>;
            type LineType: 'a
                + postgis::LineString<'a, ItemType = Self::PointType, Iter = Self::PointIter>;
            type LineIter: Iterator<Item = &'a Self::LineType>
                + ExactSizeIterator<Item = &'a Self::LineType>;
            type ItemType: 'a
                + postgis::$itemtypetrait<'a, ItemType = Self::LineType, Iter = Self::LineIter>;
            type Iter: Iterator<Item = &'a Self::ItemType>
                + ExactSizeIterator<Item = &'a Self::ItemType>;
            fn as_ewkb(
                &'a self,
            ) -> $ewkbtype<
                'a,
                Self::PointType,
                Self::PointIter,
                Self::LineType,
                Self::LineIter,
                Self::ItemType,
                Self::Iter,
            >;
        }

        impl<'a, P, I, L, K, T, J> fmt::Debug for $ewkbtype<'a, P, I, L, K, T, J>
        where
            P: 'a + postgis::Point,
            I: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
            L: 'a + postgis::LineString<'a, ItemType = P, Iter = I>,
            K: 'a + Iterator<Item = &'a L> + ExactSizeIterator<Item = &'a L>,
            T: 'a + postgis::$itemtypetrait<'a, ItemType = L, Iter = K>,
            J: 'a + Iterator<Item = &'a T> + ExactSizeIterator<Item = &'a T>,
        {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, stringify!($ewkbtype))?; //TODO
                Ok(())
            }
        }

        impl<'a, P, I, L, K, T, J> EwkbWrite for $ewkbtype<'a, P, I, L, K, T, J>
        where
            P: 'a + postgis::Point,
            I: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
            L: 'a + postgis::LineString<'a, ItemType = P, Iter = I>,
            K: 'a + Iterator<Item = &'a L> + ExactSizeIterator<Item = &'a L>,
            T: 'a + postgis::$itemtypetrait<'a, ItemType = L, Iter = K>,
            J: 'a + Iterator<Item = &'a T> + ExactSizeIterator<Item = &'a T>,
        {
            fn opt_srid(&self) -> Option<i32> {
                self.srid
            }

            fn type_id(&self) -> u32 {
                $typecode | Self::wkb_type_id(&self.point_type, self.srid)
            }

            fn write_ewkb_body<W: Write + ?Sized>(&self, w: &mut W) -> Result<(), Error> {
                w.write_u32::<LittleEndian>(self.geom.$itemname().len() as u32)?;
                for geom in self.geom.$itemname() {
                    let wkb = $ewkbitemtype {
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
            type PointIter = Iter<'a, P>;
            type LineType = LineStringT<P>;
            type LineIter = Iter<'a, Self::LineType>;
            type ItemType = $itemtype<P>;
            type Iter = Iter<'a, Self::ItemType>;
            fn as_ewkb(
                &'a self,
            ) -> $ewkbtype<
                'a,
                Self::PointType,
                Self::PointIter,
                Self::LineType,
                Self::LineIter,
                Self::ItemType,
                Self::Iter,
            > {
                $ewkbtype {
                    geom: self,
                    srid: self.srid,
                    point_type: Self::PointType::point_type(),
                }
            }
        }
    };
}

geometry_container_type!(Polygon for PolygonT contains LineStringT named rings);
impl_read_for_geometry_container_type!(singletype PolygonT contains LineStringT named rings);
geometry_container_write!(Polygon and AsEwkbPolygon for PolygonT
                          to EwkbPolygon with type code 0x03,
                          contains EwkbLineString,LineStringT as LineString named rings,
                          command write_ewkb_body);

/// OGC Polygon type
pub type Polygon = PolygonT<Point>;
/// OGC PolygonZ type
pub type PolygonZ = PolygonT<PointZ>;
/// OGC PolygonM type
pub type PolygonM = PolygonT<PointM>;
/// OGC PolygonZM type
pub type PolygonZM = PolygonT<PointZM>;

geometry_container_type!(MultiLineString for MultiLineStringT contains LineStringT named lines);
impl_read_for_geometry_container_type!(multitype MultiLineStringT contains LineStringT named lines);
geometry_container_write!(MultiLineString and AsEwkbMultiLineString for MultiLineStringT
                          to EwkbMultiLineString with type code 0x05,
                          contains EwkbLineString,LineStringT as LineString named lines,
                          command write_ewkb);

/// OGC MultiLineString type
pub type MultiLineString = MultiLineStringT<Point>;
/// OGC MultiLineStringZ type
pub type MultiLineStringZ = MultiLineStringT<PointZ>;
/// OGC MultiLineStringM type
pub type MultiLineStringM = MultiLineStringT<PointM>;
/// OGC MultiLineStringZM type
pub type MultiLineStringZM = MultiLineStringT<PointZM>;

geometry_container_type!(MultiPolygon for MultiPolygonT contains PolygonT named polygons);
impl_read_for_geometry_container_type!(multitype MultiPolygonT contains PolygonT named polygons);
geometry_container_write!(multipoly MultiPolygon and AsEwkbMultiPolygon for MultiPolygonT
                          to EwkbMultiPolygon with type code 0x06,
                          contains EwkbPolygon,PolygonT as Polygon named polygons,
                          command write_ewkb);

/// OGC MultiPolygon type
pub type MultiPolygon = MultiPolygonT<Point>;
/// OGC MultiPolygonZ type
pub type MultiPolygonZ = MultiPolygonT<PointZ>;
/// OGC MultiPolygonM type
pub type MultiPolygonM = MultiPolygonT<PointM>;
/// OGC MultiPolygonZM type
pub type MultiPolygonZM = MultiPolygonT<PointZM>;

/// Generic Geometry Data Type
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub enum GeometryT<P: postgis::Point + EwkbRead> {
    Point(P),
    LineString(LineStringT<P>),
    Polygon(PolygonT<P>),
    MultiPoint(MultiPointT<P>),
    MultiLineString(MultiLineStringT<P>),
    MultiPolygon(MultiPolygonT<P>),
    GeometryCollection(GeometryCollectionT<P>),
}

impl<'a, P> postgis::Geometry<'a> for GeometryT<P>
where
    P: 'a + postgis::Point + EwkbRead,
{
    type Point = P;
    type LineString = LineStringT<P>;
    type Polygon = PolygonT<P>;
    type MultiPoint = MultiPointT<P>;
    type MultiLineString = MultiLineStringT<P>;
    type MultiPolygon = MultiPolygonT<P>;
    type GeometryCollection = GeometryCollectionT<P>;
    fn as_type(
        &'a self,
    ) -> postgis::GeometryType<
        'a,
        P,
        LineStringT<P>,
        PolygonT<P>,
        MultiPointT<P>,
        MultiLineStringT<P>,
        MultiPolygonT<P>,
        GeometryCollectionT<P>,
    > {
        use crate::ewkb::GeometryT as A;
        use crate::types::GeometryType as B;
        match *self {
            A::Point(ref geom) => B::Point(geom),
            A::LineString(ref geom) => B::LineString(geom),
            A::Polygon(ref geom) => B::Polygon(geom),
            A::MultiPoint(ref geom) => B::MultiPoint(geom),
            A::MultiLineString(ref geom) => B::MultiLineString(geom),
            A::MultiPolygon(ref geom) => B::MultiPolygon(geom),
            A::GeometryCollection(ref geom) => B::GeometryCollection(geom),
        }
    }
}

impl<P> EwkbRead for GeometryT<P>
where
    P: postgis::Point + EwkbRead,
{
    fn point_type() -> PointType {
        P::point_type()
    }
    fn read_ewkb<R: Read>(raw: &mut R) -> Result<Self, Error> {
        let byte_order = raw.read_i8()?;
        let is_be = byte_order == 0i8;

        let type_id = read_u32(raw, is_be)?;
        let mut srid: Option<i32> = None;
        if type_id & 0x20000000 == 0x20000000 {
            srid = Some(read_i32(raw, is_be)?);
        }

        let geom = match type_id & 0xff {
            0x01 => GeometryT::Point(P::read_ewkb_body(raw, is_be, type_id, srid)?),
            0x02 => {
                GeometryT::LineString(LineStringT::<P>::read_ewkb_body(raw, is_be, type_id, srid)?)
            }
            0x03 => GeometryT::Polygon(PolygonT::read_ewkb_body(raw, is_be, type_id, srid)?),
            0x04 => GeometryT::MultiPoint(MultiPointT::read_ewkb_body(raw, is_be, type_id, srid)?),
            0x05 => GeometryT::MultiLineString(MultiLineStringT::read_ewkb_body(
                raw, is_be, type_id, srid,
            )?),
            0x06 => {
                GeometryT::MultiPolygon(MultiPolygonT::read_ewkb_body(raw, is_be, type_id, srid)?)
            }
            0x07 => GeometryT::GeometryCollection(GeometryCollectionT::read_ewkb_body(
                raw, is_be, type_id, srid,
            )?),
            _ => {
                return Err(Error::Read(format!(
                    "Error reading generic geometry type - unsupported type id {}.",
                    type_id
                )))
            }
        };
        Ok(geom)
    }
    fn read_ewkb_body<R: Read>(
        _raw: &mut R,
        _is_be: bool,
        _type_id: u32,
        _srid: Option<i32>,
    ) -> Result<Self, Error> {
        panic!("Not used for generic geometry type")
    }
}

pub enum EwkbGeometry<'a, P, PI, MP, L, LI, ML, Y, YI, MY, G, GI, GC>
where
    P: 'a + postgis::Point,
    PI: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
    MP: 'a + postgis::MultiPoint<'a, ItemType = P, Iter = PI>,
    L: 'a + postgis::LineString<'a, ItemType = P, Iter = PI>,
    LI: 'a + Iterator<Item = &'a L> + ExactSizeIterator<Item = &'a L>,
    ML: 'a + postgis::MultiLineString<'a, ItemType = L, Iter = LI>,
    Y: 'a + postgis::Polygon<'a, ItemType = L, Iter = LI>,
    YI: 'a + Iterator<Item = &'a Y> + ExactSizeIterator<Item = &'a Y>,
    MY: 'a + postgis::MultiPolygon<'a, ItemType = Y, Iter = YI>,
    G: 'a
        + postgis::Geometry<
            'a,
            Point = P,
            LineString = L,
            Polygon = Y,
            MultiPoint = MP,
            MultiLineString = ML,
            MultiPolygon = MY,
            GeometryCollection = GC,
        >,
    GI: 'a + Iterator<Item = &'a G> + ExactSizeIterator<Item = &'a G>,
    GC: 'a + postgis::GeometryCollection<'a, ItemType = G, Iter = GI>,
{
    Point(EwkbPoint<'a>),
    LineString(EwkbLineString<'a, P, PI>),
    Polygon(EwkbPolygon<'a, P, PI, L, LI>),
    MultiPoint(EwkbMultiPoint<'a, P, PI>),
    MultiLineString(EwkbMultiLineString<'a, P, PI, L, LI>),
    MultiPolygon(EwkbMultiPolygon<'a, P, PI, L, LI, Y, YI>),
    GeometryCollection(EwkbGeometryCollection<'a, P, PI, MP, L, LI, ML, Y, YI, MY, G, GI, GC>),
}

pub trait AsEwkbGeometry<'a> {
    type PointType: 'a + postgis::Point + EwkbRead;
    type PointIter: Iterator<Item = &'a Self::PointType>
        + ExactSizeIterator<Item = &'a Self::PointType>;
    type MultiPointType: 'a
        + postgis::MultiPoint<'a, ItemType = Self::PointType, Iter = Self::PointIter>;
    type LineType: 'a + postgis::LineString<'a, ItemType = Self::PointType, Iter = Self::PointIter>;
    type LineIter: Iterator<Item = &'a Self::LineType>
        + ExactSizeIterator<Item = &'a Self::LineType>;
    type MultiLineType: 'a
        + postgis::MultiLineString<'a, ItemType = Self::LineType, Iter = Self::LineIter>;
    type PolyType: 'a + postgis::Polygon<'a, ItemType = Self::LineType, Iter = Self::LineIter>;
    type PolyIter: Iterator<Item = &'a Self::PolyType>
        + ExactSizeIterator<Item = &'a Self::PolyType>;
    type MultiPolyType: 'a
        + postgis::MultiPolygon<'a, ItemType = Self::PolyType, Iter = Self::PolyIter>;
    type GeomType: 'a
        + postgis::Geometry<
            'a,
            Point = Self::PointType,
            LineString = Self::LineType,
            Polygon = Self::PolyType,
            MultiPoint = Self::MultiPointType,
            MultiLineString = Self::MultiLineType,
            MultiPolygon = Self::MultiPolyType,
            GeometryCollection = Self::GeomCollection,
        >;
    type GeomIter: Iterator<Item = &'a Self::GeomType>
        + ExactSizeIterator<Item = &'a Self::GeomType>;
    type GeomCollection: 'a
        + postgis::GeometryCollection<'a, ItemType = Self::GeomType, Iter = Self::GeomIter>;
    fn as_ewkb(
        &'a self,
    ) -> EwkbGeometry<
        'a,
        Self::PointType,
        Self::PointIter,
        Self::MultiPointType,
        Self::LineType,
        Self::LineIter,
        Self::MultiLineType,
        Self::PolyType,
        Self::PolyIter,
        Self::MultiPolyType,
        Self::GeomType,
        Self::GeomIter,
        Self::GeomCollection,
    >;
}

impl<'a, P, PI, MP, L, LI, ML, Y, YI, MY, G, GI, GC> fmt::Debug
    for EwkbGeometry<'a, P, PI, MP, L, LI, ML, Y, YI, MY, G, GI, GC>
where
    P: 'a + postgis::Point,
    PI: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
    MP: 'a + postgis::MultiPoint<'a, ItemType = P, Iter = PI>,
    L: 'a + postgis::LineString<'a, ItemType = P, Iter = PI>,
    LI: 'a + Iterator<Item = &'a L> + ExactSizeIterator<Item = &'a L>,
    ML: 'a + postgis::MultiLineString<'a, ItemType = L, Iter = LI>,
    Y: 'a + postgis::Polygon<'a, ItemType = L, Iter = LI>,
    YI: 'a + Iterator<Item = &'a Y> + ExactSizeIterator<Item = &'a Y>,
    MY: 'a + postgis::MultiPolygon<'a, ItemType = Y, Iter = YI>,
    G: 'a
        + postgis::Geometry<
            'a,
            Point = P,
            LineString = L,
            Polygon = Y,
            MultiPoint = MP,
            MultiLineString = ML,
            MultiPolygon = MY,
            GeometryCollection = GC,
        >,
    GI: 'a + Iterator<Item = &'a G> + ExactSizeIterator<Item = &'a G>,
    GC: 'a + postgis::GeometryCollection<'a, ItemType = G, Iter = GI>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, stringify!(EwkbGeometry))?; //TODO
        Ok(())
    }
}

impl<'a, P, PI, MP, L, LI, ML, Y, YI, MY, G, GI, GC> EwkbWrite
    for EwkbGeometry<'a, P, PI, MP, L, LI, ML, Y, YI, MY, G, GI, GC>
where
    P: 'a + postgis::Point,
    PI: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
    MP: 'a + postgis::MultiPoint<'a, ItemType = P, Iter = PI>,
    L: 'a + postgis::LineString<'a, ItemType = P, Iter = PI>,
    LI: 'a + Iterator<Item = &'a L> + ExactSizeIterator<Item = &'a L>,
    ML: 'a + postgis::MultiLineString<'a, ItemType = L, Iter = LI>,
    Y: 'a + postgis::Polygon<'a, ItemType = L, Iter = LI>,
    YI: 'a + Iterator<Item = &'a Y> + ExactSizeIterator<Item = &'a Y>,
    MY: 'a + postgis::MultiPolygon<'a, ItemType = Y, Iter = YI>,
    G: 'a
        + postgis::Geometry<
            'a,
            Point = P,
            LineString = L,
            Polygon = Y,
            MultiPoint = MP,
            MultiLineString = ML,
            MultiPolygon = MY,
            GeometryCollection = GC,
        >,
    GI: 'a + Iterator<Item = &'a G> + ExactSizeIterator<Item = &'a G>,
    GC: 'a + postgis::GeometryCollection<'a, ItemType = G, Iter = GI>,
{
    fn opt_srid(&self) -> Option<i32> {
        match *self {
            EwkbGeometry::Point(ref ewkb) => ewkb.opt_srid(),
            EwkbGeometry::LineString(ref ewkb) => ewkb.opt_srid(),
            EwkbGeometry::Polygon(ref ewkb) => ewkb.opt_srid(),
            EwkbGeometry::MultiPoint(ref ewkb) => ewkb.opt_srid(),
            EwkbGeometry::MultiLineString(ref ewkb) => ewkb.opt_srid(),
            EwkbGeometry::MultiPolygon(ref ewkb) => ewkb.opt_srid(),
            EwkbGeometry::GeometryCollection(ref ewkb) => ewkb.opt_srid(),
        }
    }

    fn type_id(&self) -> u32 {
        match *self {
            EwkbGeometry::Point(ref ewkb) => ewkb.type_id(),
            EwkbGeometry::LineString(ref ewkb) => ewkb.type_id(),
            EwkbGeometry::Polygon(ref ewkb) => ewkb.type_id(),
            EwkbGeometry::MultiPoint(ref ewkb) => ewkb.type_id(),
            EwkbGeometry::MultiLineString(ref ewkb) => ewkb.type_id(),
            EwkbGeometry::MultiPolygon(ref ewkb) => ewkb.type_id(),
            EwkbGeometry::GeometryCollection(ref ewkb) => ewkb.type_id(),
        }
    }

    fn write_ewkb_body<W: Write + ?Sized>(&self, w: &mut W) -> Result<(), Error> {
        match *self {
            EwkbGeometry::Point(ref ewkb) => ewkb.write_ewkb_body(w),
            EwkbGeometry::LineString(ref ewkb) => ewkb.write_ewkb_body(w),
            EwkbGeometry::Polygon(ref ewkb) => ewkb.write_ewkb_body(w),
            EwkbGeometry::MultiPoint(ref ewkb) => ewkb.write_ewkb_body(w),
            EwkbGeometry::MultiLineString(ref ewkb) => ewkb.write_ewkb_body(w),
            EwkbGeometry::MultiPolygon(ref ewkb) => ewkb.write_ewkb_body(w),
            EwkbGeometry::GeometryCollection(ref ewkb) => ewkb.write_ewkb_body(w),
        }
    }
}

impl<'a, P> AsEwkbGeometry<'a> for GeometryT<P>
where
    P: 'a + postgis::Point + EwkbRead + AsEwkbPoint<'a>,
{
    type PointType = P;
    type PointIter = Iter<'a, P>;
    type MultiPointType = MultiPointT<P>;
    type LineType = LineStringT<P>;
    type LineIter = Iter<'a, Self::LineType>;
    type MultiLineType = MultiLineStringT<P>;
    type PolyType = PolygonT<P>;
    type PolyIter = Iter<'a, Self::PolyType>;
    type MultiPolyType = MultiPolygonT<P>;
    type GeomType = GeometryT<P>;
    type GeomIter = Iter<'a, Self::GeomType>;
    type GeomCollection = GeometryCollectionT<P>;
    fn as_ewkb(
        &'a self,
    ) -> EwkbGeometry<
        'a,
        Self::PointType,
        Self::PointIter,
        Self::MultiPointType,
        Self::LineType,
        Self::LineIter,
        Self::MultiLineType,
        Self::PolyType,
        Self::PolyIter,
        Self::MultiPolyType,
        Self::GeomType,
        Self::GeomIter,
        Self::GeomCollection,
    > {
        match *self {
            GeometryT::Point(ref geom) => EwkbGeometry::Point(geom.as_ewkb()),
            GeometryT::LineString(ref geom) => EwkbGeometry::LineString(geom.as_ewkb()),
            GeometryT::Polygon(ref geom) => EwkbGeometry::Polygon(geom.as_ewkb()),
            GeometryT::MultiPoint(ref geom) => EwkbGeometry::MultiPoint(geom.as_ewkb()),
            GeometryT::MultiLineString(ref geom) => EwkbGeometry::MultiLineString(geom.as_ewkb()),
            GeometryT::MultiPolygon(ref geom) => EwkbGeometry::MultiPolygon(geom.as_ewkb()),
            GeometryT::GeometryCollection(ref geom) => {
                EwkbGeometry::GeometryCollection(geom.as_ewkb())
            }
        }
    }
}

/// OGC Geometry type
pub type Geometry = GeometryT<Point>;
/// OGC GeometryZ type
pub type GeometryZ = GeometryT<PointZ>;
/// OGC GeometryM type
pub type GeometryM = GeometryT<PointM>;
/// OGC GeometryZM type
pub type GeometryZM = GeometryT<PointZM>;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug)]
pub struct GeometryCollectionT<P: postgis::Point + EwkbRead> {
    pub geometries: Vec<GeometryT<P>>,
    pub srid: Option<i32>,
}

impl<P> GeometryCollectionT<P>
where
    P: postgis::Point + EwkbRead,
{
    pub fn new() -> GeometryCollectionT<P> {
        GeometryCollectionT {
            geometries: Vec::new(),
            srid: None,
        }
    }
}

impl<'a, P> postgis::GeometryCollection<'a> for GeometryCollectionT<P>
where
    P: 'a + postgis::Point + EwkbRead,
{
    type ItemType = GeometryT<P>;
    type Iter = Iter<'a, Self::ItemType>;
    fn geometries(&'a self) -> Self::Iter {
        self.geometries.iter()
    }
}

impl<P> EwkbRead for GeometryCollectionT<P>
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
        _srid: Option<i32>,
    ) -> Result<Self, Error> {
        let mut ret = GeometryCollectionT::new();
        let size = read_u32(raw, is_be)? as usize;
        for _ in 0..size {
            let is_be = raw.read_i8()? == 0i8;

            let type_id = read_u32(raw, is_be)?;
            let mut srid: Option<i32> = None;
            if type_id & 0x20000000 == 0x20000000 {
                srid = Some(read_i32(raw, is_be)?);
            }
            let geom = match type_id & 0xff {
                0x01 => GeometryT::Point(P::read_ewkb_body(raw, is_be, type_id, srid)?),
                0x02 => GeometryT::LineString(LineStringT::<P>::read_ewkb_body(
                    raw, is_be, type_id, srid,
                )?),
                0x03 => GeometryT::Polygon(PolygonT::read_ewkb_body(raw, is_be, type_id, srid)?),
                0x04 => {
                    GeometryT::MultiPoint(MultiPointT::read_ewkb_body(raw, is_be, type_id, srid)?)
                }
                0x05 => GeometryT::MultiLineString(MultiLineStringT::read_ewkb_body(
                    raw, is_be, type_id, srid,
                )?),
                0x06 => GeometryT::MultiPolygon(MultiPolygonT::read_ewkb_body(
                    raw, is_be, type_id, srid,
                )?),
                0x07 => GeometryT::GeometryCollection(GeometryCollectionT::read_ewkb_body(
                    raw, is_be, type_id, srid,
                )?),
                _ => {
                    return Err(Error::Read(format!(
                        "Error reading generic geometry type - unsupported type id {}.",
                        type_id
                    )))
                }
            };
            ret.geometries.push(geom);
        }
        Ok(ret)
    }
}

pub struct EwkbGeometryCollection<'a, P, PI, MP, L, LI, ML, Y, YI, MY, G, GI, GC>
where
    P: 'a + postgis::Point,
    PI: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
    MP: 'a + postgis::MultiPoint<'a, ItemType = P, Iter = PI>,
    L: 'a + postgis::LineString<'a, ItemType = P, Iter = PI>,
    LI: 'a + Iterator<Item = &'a L> + ExactSizeIterator<Item = &'a L>,
    ML: 'a + postgis::MultiLineString<'a, ItemType = L, Iter = LI>,
    Y: 'a + postgis::Polygon<'a, ItemType = L, Iter = LI>,
    YI: 'a + Iterator<Item = &'a Y> + ExactSizeIterator<Item = &'a Y>,
    MY: 'a + postgis::MultiPolygon<'a, ItemType = Y, Iter = YI>,
    G: 'a
        + postgis::Geometry<
            'a,
            Point = P,
            LineString = L,
            Polygon = Y,
            MultiPoint = MP,
            MultiLineString = ML,
            MultiPolygon = MY,
            GeometryCollection = GC,
        >,
    GI: 'a + Iterator<Item = &'a G> + ExactSizeIterator<Item = &'a G>,
    GC: 'a + postgis::GeometryCollection<'a, ItemType = G, Iter = GI>,
{
    pub geom: &'a dyn postgis::GeometryCollection<'a, ItemType = G, Iter = GI>,
    pub srid: Option<i32>,
    pub point_type: PointType,
}

pub trait AsEwkbGeometryCollection<'a> {
    type PointType: 'a + postgis::Point + EwkbRead;
    type PointIter: Iterator<Item = &'a Self::PointType>
        + ExactSizeIterator<Item = &'a Self::PointType>;
    type MultiPointType: 'a
        + postgis::MultiPoint<'a, ItemType = Self::PointType, Iter = Self::PointIter>;
    type LineType: 'a + postgis::LineString<'a, ItemType = Self::PointType, Iter = Self::PointIter>;
    type LineIter: Iterator<Item = &'a Self::LineType>
        + ExactSizeIterator<Item = &'a Self::LineType>;
    type MultiLineType: 'a
        + postgis::MultiLineString<'a, ItemType = Self::LineType, Iter = Self::LineIter>;
    type PolyType: 'a + postgis::Polygon<'a, ItemType = Self::LineType, Iter = Self::LineIter>;
    type PolyIter: Iterator<Item = &'a Self::PolyType>
        + ExactSizeIterator<Item = &'a Self::PolyType>;
    type MultiPolyType: 'a
        + postgis::MultiPolygon<'a, ItemType = Self::PolyType, Iter = Self::PolyIter>;
    type GeomType: 'a
        + postgis::Geometry<
            'a,
            Point = Self::PointType,
            LineString = Self::LineType,
            Polygon = Self::PolyType,
            MultiPoint = Self::MultiPointType,
            MultiLineString = Self::MultiLineType,
            MultiPolygon = Self::MultiPolyType,
            GeometryCollection = Self::GeomCollection,
        >;
    type GeomIter: Iterator<Item = &'a Self::GeomType>
        + ExactSizeIterator<Item = &'a Self::GeomType>;
    type GeomCollection: 'a
        + postgis::GeometryCollection<'a, ItemType = Self::GeomType, Iter = Self::GeomIter>;
    fn as_ewkb(
        &'a self,
    ) -> EwkbGeometryCollection<
        'a,
        Self::PointType,
        Self::PointIter,
        Self::MultiPointType,
        Self::LineType,
        Self::LineIter,
        Self::MultiLineType,
        Self::PolyType,
        Self::PolyIter,
        Self::MultiPolyType,
        Self::GeomType,
        Self::GeomIter,
        Self::GeomCollection,
    >;
}

impl<'a, P, PI, MP, L, LI, ML, Y, YI, MY, G, GI, GC> fmt::Debug
    for EwkbGeometryCollection<'a, P, PI, MP, L, LI, ML, Y, YI, MY, G, GI, GC>
where
    P: 'a + postgis::Point,
    PI: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
    MP: 'a + postgis::MultiPoint<'a, ItemType = P, Iter = PI>,
    L: 'a + postgis::LineString<'a, ItemType = P, Iter = PI>,
    LI: 'a + Iterator<Item = &'a L> + ExactSizeIterator<Item = &'a L>,
    ML: 'a + postgis::MultiLineString<'a, ItemType = L, Iter = LI>,
    Y: 'a + postgis::Polygon<'a, ItemType = L, Iter = LI>,
    YI: 'a + Iterator<Item = &'a Y> + ExactSizeIterator<Item = &'a Y>,
    MY: 'a + postgis::MultiPolygon<'a, ItemType = Y, Iter = YI>,
    G: 'a
        + postgis::Geometry<
            'a,
            Point = P,
            LineString = L,
            Polygon = Y,
            MultiPoint = MP,
            MultiLineString = ML,
            MultiPolygon = MY,
            GeometryCollection = GC,
        >,
    GI: 'a + Iterator<Item = &'a G> + ExactSizeIterator<Item = &'a G>,
    GC: 'a + postgis::GeometryCollection<'a, ItemType = G, Iter = GI>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, stringify!(EwkbGeometryCollection))?; //TODO
        Ok(())
    }
}

impl<'a, P, PI, MP, L, LI, ML, Y, YI, MY, G, GI, GC> EwkbWrite
    for EwkbGeometryCollection<'a, P, PI, MP, L, LI, ML, Y, YI, MY, G, GI, GC>
where
    P: 'a + postgis::Point,
    PI: 'a + Iterator<Item = &'a P> + ExactSizeIterator<Item = &'a P>,
    MP: 'a + postgis::MultiPoint<'a, ItemType = P, Iter = PI>,
    L: 'a + postgis::LineString<'a, ItemType = P, Iter = PI>,
    LI: 'a + Iterator<Item = &'a L> + ExactSizeIterator<Item = &'a L>,
    ML: 'a + postgis::MultiLineString<'a, ItemType = L, Iter = LI>,
    Y: 'a + postgis::Polygon<'a, ItemType = L, Iter = LI>,
    YI: 'a + Iterator<Item = &'a Y> + ExactSizeIterator<Item = &'a Y>,
    MY: 'a + postgis::MultiPolygon<'a, ItemType = Y, Iter = YI>,
    G: 'a
        + postgis::Geometry<
            'a,
            Point = P,
            LineString = L,
            Polygon = Y,
            MultiPoint = MP,
            MultiLineString = ML,
            MultiPolygon = MY,
            GeometryCollection = GC,
        >,
    GI: 'a + Iterator<Item = &'a G> + ExactSizeIterator<Item = &'a G>,
    GC: 'a + postgis::GeometryCollection<'a, ItemType = G, Iter = GI>,
{
    fn opt_srid(&self) -> Option<i32> {
        self.srid
    }

    fn type_id(&self) -> u32 {
        0x07 | Self::wkb_type_id(&self.point_type, self.srid)
    }

    fn write_ewkb_body<W: Write + ?Sized>(&self, w: &mut W) -> Result<(), Error> {
        w.write_u32::<LittleEndian>(self.geom.geometries().len() as u32)?;

        for geom in self.geom.geometries() {
            match geom.as_type() {
                postgis::GeometryType::Point(geom) => {
                    let wkb = EwkbPoint {
                        geom,
                        srid: None,
                        point_type: self.point_type.clone(),
                    };
                    wkb.write_ewkb(w)?;
                }
                postgis::GeometryType::LineString(geom) => {
                    let wkb = EwkbLineString {
                        geom,
                        srid: None,
                        point_type: self.point_type.clone(),
                    };
                    wkb.write_ewkb(w)?;
                }
                postgis::GeometryType::Polygon(geom) => {
                    let wkb = EwkbPolygon {
                        geom,
                        srid: None,
                        point_type: self.point_type.clone(),
                    };
                    wkb.write_ewkb(w)?;
                }
                postgis::GeometryType::MultiPoint(geom) => {
                    let wkb = EwkbMultiPoint {
                        geom,
                        srid: None,
                        point_type: self.point_type.clone(),
                    };
                    wkb.write_ewkb(w)?;
                }
                postgis::GeometryType::MultiLineString(geom) => {
                    let wkb = EwkbMultiLineString {
                        geom,
                        srid: None,
                        point_type: self.point_type.clone(),
                    };
                    wkb.write_ewkb(w)?;
                }
                postgis::GeometryType::MultiPolygon(geom) => {
                    let wkb = EwkbMultiPolygon {
                        geom,
                        srid: None,
                        point_type: self.point_type.clone(),
                    };
                    wkb.write_ewkb(w)?;
                }
                postgis::GeometryType::GeometryCollection(geom) => {
                    let wkb = EwkbGeometryCollection {
                        geom,
                        srid: None,
                        point_type: self.point_type.clone(),
                    };
                    wkb.write_ewkb(w)?;
                }
            }
        }
        Ok(())
    }
}

impl<'a, P> AsEwkbGeometryCollection<'a> for GeometryCollectionT<P>
where
    P: 'a + postgis::Point + EwkbRead,
{
    type PointType = P;
    type PointIter = Iter<'a, P>;
    type MultiPointType = MultiPointT<P>;
    type LineType = LineStringT<P>;
    type LineIter = Iter<'a, Self::LineType>;
    type MultiLineType = MultiLineStringT<P>;
    type PolyType = PolygonT<P>;
    type PolyIter = Iter<'a, Self::PolyType>;
    type MultiPolyType = MultiPolygonT<P>;
    type GeomType = GeometryT<P>;
    type GeomIter = Iter<'a, Self::GeomType>;
    type GeomCollection = GeometryCollectionT<P>;
    fn as_ewkb(
        &'a self,
    ) -> EwkbGeometryCollection<
        'a,
        Self::PointType,
        Self::PointIter,
        Self::MultiPointType,
        Self::LineType,
        Self::LineIter,
        Self::MultiLineType,
        Self::PolyType,
        Self::PolyIter,
        Self::MultiPolyType,
        Self::GeomType,
        Self::GeomIter,
        Self::GeomCollection,
    > {
        EwkbGeometryCollection {
            geom: self,
            srid: self.srid,
            point_type: P::point_type(),
        }
    }
}

/// OGC GeometryCollection type
pub type GeometryCollection = GeometryCollectionT<Point>;
/// OGC GeometryCollectionZ type
pub type GeometryCollectionZ = GeometryCollectionT<PointZ>;
/// OGC GeometryCollectionM type
pub type GeometryCollectionM = GeometryCollectionT<PointM>;
/// OGC GeometryCollectionZM type
pub type GeometryCollectionZM = GeometryCollectionT<PointZM>;
