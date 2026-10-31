# Audexis for iOS

The native SwiftUI client for an Audexis cloud library. The app includes account
sign-in, library browsing, downloads, lyrics, and native playback controls.
Mobile development is in progress.

## TO DO

- playback and decoding w symphonia n cpal

## Development

Use macOS with Xcode, Rust, and Cargo installed. Open
`audexis/audexis.xcodeproj` in Xcode, select the app target and a simulator or
device, and run it. Set a development signing team when building for a device.

Install the Rust target for the destination you will use:

```sh

rustup target add aarch64-apple-ios-sim

rustup target add aarch64-apple-ios

rustup target add x86_64-apple-ios
```

The Xcode build phase runs `scripts/build-rust.sh` to compile and link
`audexis_mobile`.

## Configuration

Development config is in `audexis/Config/Development.xcconfig`:

- `AUDEXIS_DEV_HOST`: machine hosting the development services.
- `AUDEXIS_AUTH_PORT`: website authentication port, default `4321`.
- `AUDEXIS_API_PORT`: media server port, default `3000`.

Run the website and media server separately.

## Code map

Paths below are relative to `audexis/audexis/`.

| Path          | Purpose                                               |
| ------------- | ----------------------------------------------------- |
| `Auth/`       | Sign-in, token exchange, and secure session storage   |
| `Cloud/`      | API access, downloads, and the Rust core wrapper      |
| `Playback/`   | Playback, lyrics, queue UI, and system media controls |
| `Screens/`    | Main app screens                                      |
| `Components/` | Reusable SwiftUI views                                |
| `Theme/`      | Shared visual styling                                 |
