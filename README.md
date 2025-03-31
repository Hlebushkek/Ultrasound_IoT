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