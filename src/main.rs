use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::Duration;

fn main() {
    println!("Fetching developer devices");
    let developer_devices = scrape_developer_devices();
    println!("Fetching solar devices");
    let solar_devices = get_solar_devices();
    println!("Creating mapping");
    let mapping = map_devices(solar_devices, developer_devices);
    write_csv(mapping);
}

fn write_csv(mapping: Vec<MappedDevice>) {
    let mut writer = csv::Writer::from_path("developer_to_solar_devices.csv").unwrap();

    for map in mapping {
        writer.serialize(map).unwrap();
    }

    writer.flush().unwrap();
}

#[derive(Serialize)]
struct MappedDevice {
    developer_url: String,
    solar_url: Option<String>,
}

fn map_devices(
    solar_devices: Vec<SolarDevice>,
    developer_devices: Vec<DeveloperDevice>,
) -> Vec<MappedDevice> {
    let mut mapped = Vec::new();
    for dev_device in developer_devices {
        let matching_sol_device = solar_devices
            .iter()
            .find(|sol_device| sol_device.name == dev_device.name);

        match matching_sol_device {
            Some(sol_device) => mapped.push(MappedDevice {
                solar_url: Some(sol_device.url()),
                developer_url: dev_device.url,
            }),
            None => mapped.push(MappedDevice {
                developer_url: dev_device.url,
                solar_url: None,
            }),
        }
    }

    mapped
}

#[derive(Deserialize)]
struct GraphqlResponse {
    data: DevicesData,
}

#[derive(Deserialize)]
struct DevicesData {
    devices: Vec<SolarDevice>,
}

#[derive(Deserialize)]
struct SolarDevice {
    id: String,
    name: String,
}

impl SolarDevice {
    fn url(&self) -> String {
        format!("/devices/{}/", self.id)
    }
}

fn get_solar_devices() -> Vec<SolarDevice> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(1000))
        .build()
        .unwrap();
    let query = r#"
        query {
            devices {
                id
                name
            }
        }
    "#;
    let query = HashMap::from([("query", query)]);
    let res = client
        .post("https://graphql.api.keil.arm.com/")
        .json(&query)
        .send()
        .unwrap();
    let devices: GraphqlResponse = res.json().unwrap();
    devices.data.devices
}

#[derive(Debug)]
struct DeveloperDevice {
    vendor: String,
    name: String,
    url: String,
}

fn scrape_developer_devices() -> Vec<DeveloperDevice> {
    // Reads html from a file since developer.arm.com injects elements into the dom with JS.
    let raw_html = fs::read_to_string("raw.html").unwrap();
    let document = Html::parse_document(&raw_html);

    let selector = Selector::parse(".App-intro>div>div").unwrap();
    let vendor_name_selector = Selector::parse(".expander-link>h3").unwrap();
    let device_button_selector = Selector::parse(".device-button").unwrap();

    let mut developer_devices = Vec::new();

    for child in document.select(&selector) {
        let vendor_name_el = child.select(&vendor_name_selector).next();
        if vendor_name_el.is_none() {
            continue;
        }
        let vendor = vendor_name_el.unwrap().text().collect::<String>();
        let devices = child.select(&device_button_selector);

        for device in devices {
            let device_name = device.text().next().unwrap().to_string();
            let url = _build_device_url(
                _sanitise_device_name(device_name.clone()),
                _sanitise_vendor_name(vendor.clone()),
            );
            let device = DeveloperDevice {
                vendor: vendor.clone(),
                name: device_name,
                url,
            };
            developer_devices.push(device);
        }
    }

    developer_devices
}

fn _build_device_url(name: String, vendor: String) -> String {
    format!("/embedded/cmsis/cmsis-packs/devices/{vendor}/{name}")
}

fn _sanitise_device_name(name: String) -> String {
    name.replace("-", "_")
}

fn _sanitise_vendor_name(name: String) -> String {
    name.split_whitespace().collect()
}
