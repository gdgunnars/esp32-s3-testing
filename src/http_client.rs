use anyhow::{bail, Error};
use embedded_svc::{
    http::{client::Client as HttpClient, Method},
    utils::io,
};
use esp_idf_svc::http::client::{
    Configuration as HttpConfiguration, EspHttpConnection, Response as HttpResponse,
};
use log::{error, info};

pub fn get(url: impl AsRef<str>) -> Result<String, Error> {
    let config = &HttpConfiguration {
        use_global_ca_store: true,
        crt_bundle_attach: Some(esp_idf_svc::sys::esp_crt_bundle_attach),
        ..Default::default()
    };

    let mut client = HttpClient::wrap(EspHttpConnection::new(config)?);
    let headers = [("accept", "application/json")];

    let request = client.request(Method::Get, url.as_ref(), &headers)?;
    let mut response = request.submit()?;

    match response.status() {
        200..=299 => {
            info!("Success!");
            match parse_response(&mut response) {
                Ok(body) => Ok(body),
                Err(e) => {
                    error!("Failed!");
                    bail!(e);
                }
            }
        }
        _ => {
            error!("Failed!");
            bail!("Response not 200");
        }
    }
}

fn parse_response(response: &mut HttpResponse<&mut EspHttpConnection>) -> Result<String, Error> {
    let mut buf = [0u8; 1024];
    let bytes_read = io::try_read_full(response, &mut buf).map_err(|e| e.0)?;
    info!("Read {} bytes", bytes_read);
    match std::str::from_utf8(&buf[0..bytes_read]) {
        Ok(body_string) => {
            info!(
                "Response body (truncated to {} bytes): {:?}",
                buf.len(),
                body_string
            );
            Ok(body_string.to_string())
        }
        Err(e) => {
            error!("Error decoding response body: {}", e);
            bail!("Error parsing response body");
        }
    }
}
