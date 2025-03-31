target_dir="../target"
fat_simulator_lib_dir="$target_dir/ios-simulator-fat/release"

generate_ffi() {
  echo "Generating framework module mapping and FFI bindings"
  # NOTE: Convention requires the modulemap be named module.modulemap
  cargo run --bin uniffi-bindgen-swift -- $target_dir/aarch64-apple-ios/release/lib$1.a $target_dir/uniffi-xcframework-staging --swift-sources --headers --modulemap --module-name $1FFI --modulemap-filename module.modulemap
  mkdir -p ../external/UltrasoundScanning/UltrasoundScanning/ImageGen
  mv $target_dir/uniffi-xcframework-staging/*.swift ../external/UltrasoundScanning/UltrasoundScanning/ImageGen
  mv $target_dir/uniffi-xcframework-staging/module.modulemap $target_dir/uniffi-xcframework-staging/module.modulemap
}

create_fat_simulator_lib() {
  echo "Creating a fat library for x86_64 and aarch64 simulators"
  mkdir -p $fat_simulator_lib_dir
  lipo -create $target_dir/x86_64-apple-ios/release/lib$1.a $target_dir/aarch64-apple-ios-sim/release/lib$1.a -output $fat_simulator_lib_dir/lib$1.a
}

build_xcframework() {
  # Builds an XCFramework
  echo "Generating XCFramework"
  rm -rf $target_dir/ios  # Delete the output folder so we can regenerate it
  xcodebuild -create-xcframework \
    -library $target_dir/aarch64-apple-ios/release/lib$1.a -headers $target_dir/uniffi-xcframework-staging \
    -library $target_dir/ios-simulator-fat/release/lib$1.a -headers $target_dir/uniffi-xcframework-staging \
    -output $target_dir/ios/lib$1-rs.xcframework

  if $release; then
    echo "Building xcframework archive"
    ditto -c -k --sequesterRsrc --keepParent $target_dir/ios/lib$1-rs.xcframework $target_dir/ios/lib$1-rs.xcframework.zip
    checksum=$(swift package compute-checksum $target_dir/ios/lib$1-rs.xcframework.zip)
    version=$(cargo metadata --format-version 1 | jq -r --arg pkg_name "$1" '.packages[] | select(.name==$pkg_name) .version')
    sed -i "" -E "s/(let releaseTag = \")[^\"]+(\")/\1$version\2/g" ../Package.swift
    sed -i "" -E "s/(let releaseChecksum = \")[^\"]+(\")/\1$checksum\2/g" ../Package.swift
  fi
}


basename=ultrasound
p_basename=$basename-iot-image-gen

cargo build -p $p_basename --lib --release --target x86_64-apple-ios
cargo build -p $p_basename --lib --release --target aarch64-apple-ios-sim
cargo build -p $p_basename --lib --release --target aarch64-apple-ios

generate_ffi $basename
create_fat_simulator_lib $basename
build_xcframework $basename