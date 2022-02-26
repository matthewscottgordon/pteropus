use {
    ash::{vk, Entry},
    clap::Parser,
    geo_types::Point,
    las::Read as LasRead,
    proj::Proj,
    tokio::io::AsyncReadExt,
};

mod geonb;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about=None)]
struct Args {
    // Latitude to fetch LIDAR tile at
    #[clap(long)]
    latitude: Option<f64>,

    // Longitude to fetch LIDAR tile at
    #[clap(long)]
    longitude: Option<f64>,
}

fn pretty_print_memory_size(x: u64) -> String {
    if x > 1_000_000_000 {
        format!("{:.1}G", x / 1_000_000_000)
    } else if x > 1_000_000 {
        format!("{:.0}M", x / 1_000_000)
    } else if x > 1000 {
        format!("{:.0}K", x / 1000)
    } else {
        format!("{}", x)
    }
}

fn init_vulkan() {
    let instance = {
        let entry = Entry::linked();
        let app_info = vk::ApplicationInfo {
            api_version: vk::make_api_version(0, 1, 0, 0),
            ..Default::default()
        };
        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            ..Default::default()
        };
        unsafe {
            entry
                .create_instance(&create_info, None)
                .expect("vulkan instance")
        }
    };

    unsafe {
        instance
            .enumerate_physical_devices()
            .expect("vulkan physical devices")
            .iter()
            .for_each(|&device| {
                let device_properties = instance.get_physical_device_properties(device);
                let api_version = device_properties.api_version;
                let api_major_version = (api_version >> 22) & 0x7f;
                let api_minor_version = (api_version >> 12) & 0x3ff;
                println!(
                    "{}:\n\tAPI Version{}.{}\n\t{:?}",
                    std::ffi::CStr::from_ptr(&device_properties.device_name[0])
                        .to_str()
                        .expect("device name string"),
                    api_major_version,
                    api_minor_version,
                    device_properties.device_type
                );
                println!("\tMemory:");
                let memory_properties = instance.get_physical_device_memory_properties(device);
                for i in 0..memory_properties.memory_type_count as usize {
                    let memory_type = memory_properties.memory_types[i];
                    println!("\t\t{:?}", memory_type.property_flags);
                    let heap_index = memory_type.heap_index as usize;
                    let heap = memory_properties.memory_heaps[heap_index];
                    println!(
                        "\t\t\t{}: {}\t{:?}",
                        heap_index,
                        pretty_print_memory_size(heap.size),
                        heap.flags
                    );
                }
                println!("\tQueues:");
                instance
                    .get_physical_device_queue_family_properties(device)
                    .iter()
                    .enumerate()
                    .for_each(|(i, queue_info)| {
                        println!(
                            "\t\t{}: {:?} ({})",
                            i, queue_info.queue_flags, queue_info.queue_count
                        );
                    });
            });
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    init_vulkan();

    if let (Some(latitude), Some(longitude)) = (args.latitude, args.longitude) {
        let location = Proj::new_known_crs("+proj=longlat +datum=WGS84", "EPSG:2953", None)
            .unwrap()
            .convert(Point::new(longitude, latitude))
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
    }

    Ok(())
}
