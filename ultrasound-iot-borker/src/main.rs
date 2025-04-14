use std::collections::HashMap;
use std::time::Duration;

use reqwest::header::CONTENT_TYPE;
use tracing::debug;
use uuid::Uuid;

use rumqttc::{AsyncClient, Event, Incoming, MqttOptions, QoS};

use ultrasound_iot_borker::message::*;
use ultrasound_iot_borker::settings::Settings;

const SAMPLE_SIZE: usize = 5;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();

    let settings = Settings::new().expect("Failed to initialize settings");
    debug!("{:?}", settings);

    let mut mqttoptions = MqttOptions::new("6_2_hub", settings.broker.host, settings.broker.port);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut connection) = AsyncClient::new(mqttoptions, 10);
    client
        .subscribe("rust_6_project/device/+/session/+", QoS::AtMostOnce)
        .await
        .unwrap();

    let client = reqwest::Client::new();
    let url = settings.server.url.clone();

    let mut device_progress: HashMap<Uuid, Vec<f32>> = HashMap::new();

    while let Ok(notification) = connection.poll().await {
        debug!("Received = {:?}", notification);

        let publish = match notification {
            Event::Incoming(Incoming::Publish(publish)) => publish,
            _ => continue,
        };

        // Extract device and session
        let topic_parts: Vec<&str> = publish.topic.split('/').collect();
        debug!("topic_parts = {:?}", topic_parts);
        if topic_parts.len() < 5 {
            continue;
        }

        let device = match Uuid::parse_str(topic_parts[2]) {
            Ok(device) => device,
            Err(_) => {
                continue;
            }
        };

        let session = topic_parts[4].to_owned();

        let msg = match serde_json::from_slice::<DeviceMessage>(&publish.payload) {
            Ok(msg) => msg,
            Err(_) => {
                continue;
            }
        };

        device_progress
            .entry(device)
            .or_insert(Vec::with_capacity(SAMPLE_SIZE))
            .push(msg.value);

        debug!("{:?}", device_progress);
        if device_progress.get(&device).unwrap().len() < SAMPLE_SIZE {
            continue;
        }
        let values = device_progress.remove(&device).unwrap();

        let scan_message = ScanMessage {
            device,
            session,
            values,
        };
        debug!("{:?}", scan_message);

        let json_scan_message =
            serde_json::to_string(&scan_message).expect("Failed to serialize scan message");
        let _ = client
            .post(&url)
            .header(CONTENT_TYPE, "application/json")
            .body(json_scan_message)
            .send()
            .await?;
    }

    Ok(())
}
