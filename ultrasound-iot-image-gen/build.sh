target_dir="../target"
fat_simulator_lib_dir="$target_dir/ios-simulator-fat/release"
fat_macOS_lib_dir="$target_dir/macOS-fat/release"

generate_ffi() {
  echo "Generating framework module mapping and FFI bindings"
  # NOTE: Convention requires the modulemap be named module.modulemap
  cargo run --bin uniffi-bindgen-swift -- $target_dir/aarch64-apple-darwin/release/lib$1.a $target_dir/uniffi-xcframework-staging --swift-sources --headers --modulemap --module-name $1FFI --modulemap-filename module.modulemap
  mkdir -p ../external/UltrasoundScanning/UltrasoundScanning/ImageGen
  mv $target_dir/uniffi-xcframework-staging/*.swift ../external/UltrasoundScanning/UltrasoundScanning/ImageGen
  mv $target_dir/uniffi-xcframework-staging/module.modulemap $target_dir/uniffi-xcframework-staging/module.modulemap
}

create_fat_simulator_lib() {
  echo "Creating a fat library for x86_64 and aarch64 simulators"
  mkdir -p $fat_simulator_lib_dir
  lipo -create $target_dir/x86_64-apple-ios/release/lib$1.a $target_dir/aarch64-apple-ios-sim/release/lib$1.a -output $fat_simulator_lib_dir/lib$1.a
}

create_fat_macos_lib() {
  echo "Creating a fat library for x86_64 and aarch64 macOS"
  mkdir -p $fat_macOS_lib_dir
  lipo -create $target_dir/x86_64-apple-darwin/release/lib$1.a $target_dir/aarch64-apple-darwin/release/lib$1.a -output $fat_macOS_lib_dir/lib$1.a
}

build_xcframework() {
  # Builds an XCFramework
  echo "Generating XCFramework"
  rm -rf $target_dir/swift  # Delete the output folder so we can regenerate it
  xcodebuild -create-xcframework \
    -library $target_dir/aarch64-apple-ios/release/lib$1.a -headers $target_dir/uniffi-xcframework-staging \
    -library $target_dir/ios-simulator-fat/release/lib$1.a -headers $target_dir/uniffi-xcframework-staging \
    -library $target_dir/macOS-fat/release/lib$1.a -headers $target_dir/uniffi-xcframework-staging \
    -output $target_dir/swift/lib$1-rs.xcframework
}


basename=ultrasound
p_basename=$basename-iot-image-gen

cargo build -p $p_basename --lib --release --target x86_64-apple-ios
cargo build -p $p_basename --lib --release --target aarch64-apple-ios-sim

cargo build -p $p_basename --lib --release --target aarch64-apple-ios

cargo build -p $p_basename --lib --release --target x86_64-apple-darwin
cargo build -p $p_basename --lib --release --features rf2iq --target aarch64-apple-darwin

generate_ffi $basename
create_fat_simulator_lib $basename
create_fat_macos_lib $basename
build_xcframework $basename