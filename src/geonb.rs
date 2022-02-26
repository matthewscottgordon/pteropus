use {futures::stream::TryStreamExt, geo_types::Point, tokio_util::io::StreamReader};

#[derive(Debug)]
pub struct Error {
    pub message: String,
}

impl Error {
    pub fn new(message: &str) -> Error {
        let message = message.to_string();
        Error { message }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "GeoNB Error: \"{}\"", self.message)
    }
}

impl std::error::Error for Error {}
fn convert_err(err: reqwest::Error) -> std::io::Error {
    todo!()
}
pub async fn get_lidar_tile_around_point(
    location: Point<f64>,
) -> Result<impl tokio::io::AsyncRead, anyhow::Error> {
    let client = reqwest::Client::new();
    let query_response_json = &client
        .get("https://geonb.snb.ca/arcgis/rest/services/GeoNB_SNB_LidarIndex/MapServer/1/query")
        .query(&[
            ("f", "json"),
            ("geometryType", "esriGeometryPoint"),
            ("geometry", &format!("{},{}", location.x(), location.y())),
            ("returnIdsOnly", "true"),
        ])
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&query_response_json).expect("JSON")
    );

    let object_id = query_response_json
        .get("objectIds")
        .and_then(|object_ids| object_ids.get(0))
        .and_then(&serde_json::Value::as_i64)
        .ok_or_else(|| Error::new("Could not find \"objectId\" in response."))?;

    let object_response_json = client
        .get(format!(
            "https://geonb.snb.ca/arcgis/rest/services/GeoNB_SNB_LidarIndex/MapServer/1/{}",
            object_id
        ))
        .query(&[("f", "json")])
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    println!(
        "{}",
        serde_json::to_string_pretty(&object_response_json).expect("JSON")
    );

    let laz_file_url = object_response_json
        .get("feature")
        .ok_or_else(|| Error::new("Could not find \"feature\" in response."))?
        .get("attributes")
        .ok_or_else(|| Error::new("Could not find \"attributes\" in response."))?
        .get("FILE_URL")
        .ok_or_else(|| Error::new("Could not find \"FILE_URL\" in response."))?
        .as_str()
        .ok_or_else(|| Error::new("Expected \"FILE_URL\" to be a string but it was not."))?;

    println!("LAZ URL: {}", laz_file_url);
    Ok(StreamReader::new(
        client
            .get(laz_file_url)
            .send()
            .await?
            .bytes_stream()
            .map_err(convert_err),
    ))
}

pub async fn test() -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();
    print!(
        "{}",
        serde_json::to_string_pretty(&client
            .get("https://geonb.snb.ca/arcgis/rest/services/GeoNB_SNB_LidarIndex/MapServer/1/query")
            .query(&[("f", "json"),("geometryType","esriGeometryPoint"),("geometry","2470000,7443000"),("returnIdsOnly","true")])
            .send()
            .await?
            .json::<serde_json::Value>().await?).expect("JSON")
    );
    print!(
        "{}",
        serde_json::to_string_pretty(&client
            .get("https://geonb.snb.ca/arcgis/rest/services/GeoNB_SNB_LidarIndex/MapServer/1/14601")
            .query(&[("f", "json")])
            .send()
            .await?
            .json::<serde_json::Value>().await?).expect("JSON")
    );
    Ok(())
}
