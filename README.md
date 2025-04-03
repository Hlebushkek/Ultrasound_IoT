# Ultrasound_IoT
IoT system for processing mock ultrasound data

## embedded
- Responsible for emmiting periodical mock messages via mqtt
- Have pre-defined device_id and generate scan session_id

Used examples from https://github.com/embassy-rs/embassy/tree/main/embassy-boot-rp

## broker
- MQTT broker via public HiveMQ
- Aggregate several embedded device messages and then submit them to server

## server
- Manage patient and their scans in db
- Distirbute scan files
- Establish websocket connection with clients to send scan session updates

## image-gen
- Rust lib for ultrasound raw data convertion to image
- converts IQ data to image
- feature flag "rf2iq" enables convertion of RF data to IQ (only used for macOS targets, as hdf5 is not cross-compiled)
- UniFFI bindgen for Swift
- Example used from https://github.com/ianthetechie/uniffi-starter and https://github.com/csheaff/us-beamform-linarray
- Run build.sh for .xcframework creation

## external/UltrasoundScanningApp
- SwiftUI cross-platofrm application
- Establosh websocket connection with server to receive session updates
- Allows to finalize session with specifing "patient" uuid
- Uses image-gen lib wrapped into xcframework to convert process downloaded .h5 file