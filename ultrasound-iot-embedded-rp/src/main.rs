//! This example test the RP Pico W on board LED.
//!
//! It does not work with the RP Pico board. See blinky.rs.

#![no_std]
#![no_main]

use cyw43::JoinOptions;
use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::clocks::RoscRng;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::pio::Pio;
use embassy_rp::adc::{Adc, Channel, Config as EmbassyAdcConfig};
use embassy_time::{Duration, Timer};
use embassy_net::{Config, StackResources};
use embassy_net::tcp::TcpSocket;
use embassy_net::{dns::DnsQueryType, Config as EmbassyNetConfig};
use rust_mqtt::{
    client::{client::MqttClient, client_config::ClientConfig},
    packet::v5::reason_codes::ReasonCode,
    utils::rng_generator::CountingRng,
};
use heapless::String;
use rand::RngCore;
use core::fmt::Write;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => embassy_rp::adc::InterruptHandler;
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<PIO0>;
});

const WIFI_NETWORK: &str = env!("WIFI_NETWORK");
const WIFI_PASSWORD: &str = env!("WIFI_PASSWORD");

const DEVICE_ID: &str = env!("DEVICE_ID");

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let mut adc = Adc::new(p.ADC, Irqs, EmbassyAdcConfig::default());

    let mut rng = RoscRng;

    let fw = include_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = include_bytes!("../cyw43-firmware/43439A0_clm.bin");

    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download ../../cyw43-firmware/43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download ../../cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    //let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    //let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let mut ts = Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        p.DMA_CH0,
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();
    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    unwrap!(spawner.spawn(cyw43_task(runner)));

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());

    // Generate random seed
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<3>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(net_device, config, RESOURCES.init(StackResources::new()), seed);

    unwrap!(spawner.spawn(net_task(runner)));

    loop {
        match control
            .join(WIFI_NETWORK, JoinOptions::new(WIFI_PASSWORD.as_bytes()))
            .await
        {
            Ok(_) => break,
            Err(err) => {
                error!("join failed with status={}", err.status);
                return;
            }
        }
    }

    // Wait for DHCP, not necessary when using static IP
    info!("waiting for DHCP...");
    while !stack.is_config_up() {
        Timer::after_millis(100).await;
    }
    info!("DHCP is now up!");

    info!("waiting for link up...");
    while !stack.is_link_up() {
        Timer::after_millis(500).await;
    }
    info!("Link is up!");

    info!("waiting for stack to be up...");
    stack.wait_config_up().await;
    info!("Stack is up!");

    let mut rx_buffer = [0; 256];
    let mut tx_buffer = [0; 256];
    let mut buf = [0; 256];

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    let address = match stack
        .dns_query("broker.hivemq.com", DnsQueryType::A)
        .await
        .map(|a| a[0])
    {
        Ok(address) => address,
        Err(e) => {
            error!("DNS lookup error: {:?}", e);
            return;
        }
    };

    let remote_endpoint = (address, 1883);
    info!("connecting...");
    let connection = socket.connect(remote_endpoint).await;
    if let Err(e) = connection {
        error!("connect error: {:?}", e);
        return;
    }
    info!("connected!");

    let mut config = ClientConfig::new(
        rust_mqtt::client::client_config::MqttVersion::MQTTv5,
        CountingRng(20000),
    );
    config.add_max_subscribe_qos(rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1);
    config.add_client_id(DEVICE_ID);
    config.max_packet_size = 100;
    let mut recv_buffer = [0; 256];
    let mut write_buffer = [0; 256];

    let mut client =
        MqttClient::<_, 5, _>::new(socket, &mut write_buffer, 256, &mut recv_buffer, 256, config);

    match client.connect_to_broker().await {
        Ok(()) => {}
        Err(mqtt_error) => match mqtt_error {
            ReasonCode::NetworkError => {
                error!("MQTT Network Error");
                return;
            }
            _ => {
                error!("Other MQTT Error: {:?}", mqtt_error);
                return;
            }
        },
    }

    let session_id = generate_id_hex(&mut rng);
    let mut topic: String<128> = String::new();
    let _ = core::fmt::write(&mut topic, format_args!("rust_6_project/device/{}/session/{}", DEVICE_ID, &session_id));

    let delay = Duration::from_secs(1);
    loop {
        info!("led on!");
        control.gpio_set(0, true).await;
        Timer::after(delay).await;

        let temp = adc.read(&mut ts).await.unwrap();
        let celsius = convert_to_celsius(temp);

        let mut celsius_string = String::<64>::new();
        let _ = core::fmt::write(&mut celsius_string, format_args!("{{ value: {} }}", celsius));

        match client
            .send_message(
                &topic,
                celsius_string.as_bytes(),
                rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS1,
                true,
            )
            .await
        {
            Ok(()) => {}
            Err(mqtt_error) => match mqtt_error {
                ReasonCode::NetworkError => {
                    error!("MQTT Network Error");
                    return;
                }
                _ => {
                    error!("Other MQTT Error: {:?}", mqtt_error);
                    return;
                }
            },
        }

        info!("led off!");
        control.gpio_set(0, false).await;
        Timer::after(delay).await;
    }
}

fn generate_id(rng: &mut impl RngCore) -> [u8; 8] {
    let mut buffer = [0u8; 8];
    rng.fill_bytes(&mut buffer);
    buffer
}

fn generate_id_hex(rng: &mut impl RngCore) -> heapless::String<16> {
    let id = generate_id(rng);
    let mut s = heapless::String::<16>::new();
    for byte in &id {
        let _ = s.write_fmt(format_args!("{:02x}", byte));
    }
    s
}

fn convert_to_celsius(raw_temp: u16) -> f32 {
    // According to chapter 4.9.5. Temperature Sensor in RP2040 datasheet
    let temp = 27.0 - (raw_temp as f32 * 3.3 / 4096.0 - 0.706) / 0.001721;
    let sign = if temp < 0.0 { -1.0 } else { 1.0 };
    let rounded_temp_x10: i16 = ((temp * 10.0) + 0.5 * sign) as i16;
    (rounded_temp_x10 as f32) / 10.0
}