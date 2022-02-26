use {clap::Parser, geo_types::Point, las::Read as LasRead, proj::Proj, tokio::io::AsyncReadExt};

mod geonb;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about=None)]
struct Args {
    // Latitude to fetch LIDAR tile at
    #[clap(long)]
    latitude: f64,

    // Longitude to fetch LIDAR tile at
    #[clap(long)]
    longitude: f64,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let location = Proj::new_known_crs("+proj=longlat +datum=WGS84", "EPSG:2953", None)
        .unwrap()
        .convert(Point::new(args.longitude, args.latitude))
        .unwrap();
    println!("{:?}", location);
    let mut las_reader =
        tokio::io::BufReader::new(geonb::get_lidar_tile_around_point(location).await?);
    let mut las_bytes = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut byte_count = 0;
    loop {
        let num_bytes = las_reader.read(&mut buffer).await?;
        if num_bytes == 0 {
            break;
        }
        byte_count += num_bytes;
        print!("{} bytes read\r", byte_count);
        las_bytes.extend_from_slice(&buffer[0..num_bytes]);
    }
    println!();
    let mut las_reader = las::Reader::new(std::io::Cursor::new(las_bytes))?;
    for wrapped_point in las_reader.points().take(10) {
        let point = wrapped_point.unwrap();
        println!("Point coordinates: ({}, {}, {})", point.x, point.y, point.z);
    }

    Ok(())
}
