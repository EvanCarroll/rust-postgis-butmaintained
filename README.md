postgis-butmaintained
============

[![Build Status](https://travis-ci.org/andelf/rust-postgis.svg?branch=master)](https://travis-ci.org/andelf/rust-postgis)
[![Crates.io](https://meritbadge.herokuapp.com/postgis)](https://crates.io/crates/postgis)

[Documentation](https://docs.rs/postgis/)

An extension to rust-postgres, adds support for PostGIS.

- PostGIS type helper
- GCJ02 support (used offically in Mainland China)
- Tiny WKB (TWKB) support
- Optional serialization/deserialization support via serde

## Usage

```rust
use postgres::{Client, NoTls};
use postgis::{ewkb, LineString};

fn main() {
    let mut client = Client::connect("host=localhost user=postgres", NoTls).unwrap();
    for row in &client.query("SELECT * FROM busline", &[]).unwrap() {
        let route: ewkb::LineString = row.get("route");
        let last_stop = route.points().last().unwrap();
        let _ = client.execute("INSERT INTO stops (stop) VALUES ($1)", &[&last_stop]);
    }
}
```

Handling NULL values:
```rust
let route = row.try_get::<_, Option<ewkb::LineString>>("route");
match route {
    Ok(Some(geom)) => { println!("{:?}", geom) }
    Ok(None) => { /* Handle NULL value */ }
    Err(err) => { println!("Error: {}", err) }
}
```

## Writing other geometry types into PostGIS

rust-postgis supports writing geometry types into PostGIS which implement the following traits:

* `Point`, `LineString`, ...
* `AsEwkbPoint`, `AsEwkbLineString`, ...

See the TWKB implementation as an example.

An example for reading a TWKB geometry and writing it back as EWKB:

```rust
use postgis::twkb;
use postgis::LineString;

for row in &conn.query("SELECT ST_AsTWKB(route) FROM busline", &[]).unwrap() {
    let route: twkb::LineString = row.get(0);
    let last_stop = route.points().last().unwrap();
    let _ = conn.execute("INSERT INTO stops (stop) VALUES ($1)", &[&last_stop.as_ewkb()]);
}
```


## Unit tests

Unit tests which need a PostgreSQL connection are ignored by default.
To run the database tests, declare the connection in an environment variable `DBCONN`. Example:

    export DBCONN=postgresql://user@localhost/testdb

Run the tests with

    cargo test -- --ignored
    
## Serialization with serde

This crate provides optional support for serializing and deserializing geometry types using serde. To enable this feature, add the following to your `Cargo.toml`:

```toml
[dependencies]
postgis = { version = "0.9.0", features = ["serde"] }
```

With this feature enabled, all geometry types in `ewkb` and `twkb` modules will implement `Serialize` and `Deserialize` traits:

```rust
use postgis::ewkb::Point;
use serde_json;

let point = Point {
    x: 1.0,
    y: 2.0,
    srid: Some(4326),
};

// Serialize to JSON
let json = serde_json::to_string(&point).unwrap();
println!("{}", json);

// Deserialize from JSON
let deserialized: Point = serde_json::from_str(&json).unwrap();
```
