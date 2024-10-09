use anyhow::{bail, Error};
use embedded_svc::http::{client::Client as HttpClient, Method};
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
    let mut buf = Vec::new(); // A dynamic buffer allocated on the heap
    let mut tmp_buf = [0u8; 256]; // A smaller temporary buffer for each chunk

    // Read the response in chunks
    loop {
        // Attempt to read a chunk of data into the temporary buffer
        match response.read(&mut tmp_buf) {
            Ok(0) => break, // No more data to read (EOF)
            Ok(bytes_read) => {
                // Append the read bytes to the buffer
                buf.extend_from_slice(&tmp_buf[..bytes_read]);
                info!("Read {} bytes", bytes_read);
            }
            Err(e) => {
                error!("Error reading response: {}", e);
                bail!("Error reading response");
            }
        }
    }

    // Convert the accumulated bytes in `buf` to a UTF-8 string
    match String::from_utf8(buf) {
        Ok(body_string) => {
            info!("Full response body: {:?}", body_string);
            Ok(body_string)
        }
        Err(e) => {
            error!("Error decoding response body: {}", e);
            bail!("Error parsing response body");
        }
    }
}
