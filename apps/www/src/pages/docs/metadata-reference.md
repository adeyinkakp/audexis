---
layout: ../../layouts/MarkdownLayout.astro
title: "Audexis — Metadata field reference"
description:
  "The regular field catalog, supported value types, and specialized metadata
  editors."
---

# Metadata field reference

This reference lists the logical fields in Audexis’s current backend catalog.
These identifiers describe Audexis fields, not literal native tag names to type
into another program.

## Find the right kind of field

For a recognized regular field, use **Settings → Metadata** to add it to
Details. For a name of your own, use **Settings → Custom Fields**. The Songs
header menu independently controls columns; it does not control the editor’s
field selection.

A field can be listed in the catalog without having an editable native
representation in every format. Available text fields may use a named
custom-text fallback. Specialized and binary entries are not ordinary text
controls.

## Values and identifiers

The tables show the backend’s logical `FrameKey` identifier and its user-facing
label. Native representations can differ: for example, a field can use an ID3
frame in MP3 and a named comment in FLAC. Do not assume that the logical
identifier is the exact storage key.

“Multiple” identifies fields for which the catalog offers multiple values. Enter
those in separate inputs. Do not use semicolons as a delimiter unless you
intentionally want a literal semicolon in one value.

Rating, play-count, ReplayGain, gapless, and other technical metadata fields are
not a promise that Audexis uses their values to control playback. Audexis’s own
favorites and listening records are stored separately. The equalizer is
configured through its own interface.

## Regular text fields

| Logical field                 | Display label                  | Multiple |
| ----------------------------- | ------------------------------ | -------- |
| `AcoustidFingerprint`         | AcoustID Fingerprint           | No       |
| `AcoustidId`                  | AcoustID Id                    | No       |
| `Album`                       | Album                          | No       |
| `AlbumArtist`                 | Album Artist                   | Yes      |
| `AlbumArtistSort`             | Album Artist Sort              | No       |
| `AlbumSort`                   | Album Sort                     | No       |
| `Arranger`                    | Arranger                       | No       |
| `Artist`                      | Artist                         | Yes      |
| `ArtistSort`                  | Artist Sort                    | No       |
| `Artists`                     | Artists                        | No       |
| `Asin`                        | Asin                           | No       |
| `Barcode`                     | Barcode                        | No       |
| `BeatsPerMinute`              | Beats Per Minute               | No       |
| `CatalogNumber`               | Catalog Number                 | No       |
| `Comments`                    | Comments                       | Yes      |
| `Compilation`                 | Compilation                    | No       |
| `Composer`                    | Composer                       | Yes      |
| `ComposerSort`                | Composer Sort                  | No       |
| `Conductor`                   | Conductor                      | No       |
| `ContentGroup`                | Content Group                  | No       |
| `Director`                    | Director                       | No       |
| `DiscNumber`                  | Disc Number                    | No       |
| `DiscSubtitle`                | Disc Subtitle                  | No       |
| `EncodedBy`                   | Encoded By                     | No       |
| `EncoderSettings`             | Encoder Settings               | No       |
| `Engineer`                    | Engineer                       | No       |
| `FileType`                    | File Type                      | No       |
| `Gapless`                     | Gapless                        | No       |
| `Genre`                       | Genre                          | Yes      |
| `Grouping`                    | Grouping                       | No       |
| `InitialKey`                  | Initial Key                    | No       |
| `Isrc`                        | Isrc                           | No       |
| `Label`                       | Label                          | No       |
| `Language`                    | Language                       | No       |
| `Length`                      | Length                         | No       |
| `License`                     | License                        | No       |
| `Lyricist`                    | Lyricist                       | Yes      |
| `Lyrics`                      | Lyrics                         | No       |
| `Media`                       | Media                          | No       |
| `MediaType`                   | Media Type                     | No       |
| `Mixer`                       | Mixer                          | No       |
| `Mood`                        | Mood                           | No       |
| `Movement`                    | Movement                       | No       |
| `MovementNumber`              | Movement Number                | No       |
| `MovementTotal`               | Movement Total                 | No       |
| `MusicIpFingerprint`          | Music Ip Fingerprint           | No       |
| `MusicIpPuid`                 | Music Ip Puid                  | No       |
| `MusicBrainzAlbumArtistId`    | MusicBrainz Album Artist Id    | No       |
| `MusicBrainzAlbumId`          | MusicBrainz Album Id           | No       |
| `MusicBrainzArtistId`         | MusicBrainz Artist Id          | No       |
| `MusicBrainzDiscId`           | MusicBrainz Disc Id            | No       |
| `MusicBrainzOriginalAlbumId`  | MusicBrainz Original Album Id  | No       |
| `MusicBrainzOriginalArtistId` | MusicBrainz Original Artist Id | No       |
| `MusicBrainzRecordingId`      | MusicBrainz Recording Id       | No       |
| `MusicBrainzReleaseGroupId`   | MusicBrainz Release Group Id   | No       |
| `MusicBrainzReleaseTrackId`   | MusicBrainz Release Track Id   | No       |
| `MusicBrainzTrackId`          | MusicBrainz Track Id           | No       |
| `MusicBrainzTrmId`            | MusicBrainz Trm Id             | No       |
| `MusicBrainzWorkId`           | MusicBrainz Work Id            | No       |
| `OriginalAlbum`               | Original Album                 | No       |
| `OriginalArtist`              | Original Artist                | No       |
| `OriginalDate`                | Original Date                  | No       |
| `OriginalFilename`            | Original Filename              | No       |
| `OriginalYear`                | Original Year                  | No       |
| `Performer`                   | Performer                      | No       |
| `PlayCount`                   | Play Count                     | No       |
| `Podcast`                     | Podcast                        | No       |
| `Producer`                    | Producer                       | No       |
| `Rating`                      | Rating                         | No       |
| `RecordingDate`               | Recording Date                 | No       |
| `ReleaseCountry`              | Release Country                | No       |
| `ReleaseDate`                 | Release Date                   | No       |
| `ReleaseStatus`               | Release Status                 | No       |
| `ReleaseType`                 | Release Type                   | No       |
| `Remixer`                     | Remixer                        | No       |
| `ReplayGainAlbumGain`         | Replay Gain Album Gain         | No       |
| `ReplayGainAlbumPeak`         | Replay Gain Album Peak         | No       |
| `ReplayGainAlbumRange`        | Replay Gain Album Range        | No       |
| `ReplayGainReferenceLoudness` | Replay Gain Reference Loudness | No       |
| `ReplayGainTrackGain`         | Replay Gain Track Gain         | No       |
| `ReplayGainTrackPeak`         | Replay Gain Track Peak         | No       |
| `ReplayGainTrackRange`        | Replay Gain Track Range        | No       |
| `Script`                      | Script                         | No       |
| `Show`                        | Show                           | No       |
| `ShowMovement`                | Show Movement                  | No       |
| `ShowSort`                    | Show Sort                      | No       |
| `SoftwareEncoder`             | Software Encoder               | No       |
| `Subtitle`                    | Subtitle                       | No       |
| `SynchronizedLyrics`          | Synchronized Lyrics            | No       |
| `Time`                        | Time                           | No       |
| `Title`                       | Title                          | No       |
| `TitleSort`                   | Title Sort                     | No       |
| `TotalDiscs`                  | Total Discs                    | No       |
| `TotalTracks`                 | Total Tracks                   | No       |
| `TrackNumber`                 | Track Number                   | No       |
| `UnsyncedLyrics`              | Unsynced Lyrics                | No       |
| `Work`                        | Work                           | No       |
| `Writer`                      | Writer                         | No       |
| `Year`                        | Year                           | No       |

## Regular URL fields

| Logical field     | Display label     | Multiple |
| ----------------- | ----------------- | -------- |
| `ArtistURL`       | Artist URL        | No       |
| `AudioFileURL`    | Audio File URL    | No       |
| `BitmapImageURL`  | Bitmap Image URL  | No       |
| `CommercialURL`   | Commercial URL    | No       |
| `CopyrightURL`    | Copyright URL     | No       |
| `PaymentURL`      | Payment URL       | No       |
| `PodcastUrl`      | Podcast Url       | No       |
| `RadioStationURL` | Radio Station URL | No       |
| `Website`         | Website           | No       |

## Custom and artwork fields

Use the Custom Fields tab for text/URL entries and the Artwork tab for images.

| Logical field     | Display label     | Multiple |
| ----------------- | ----------------- | -------- |
| `AttachedPicture` | Attached Picture  | Yes      |
| `UserDefinedText` | User Defined Text | Yes      |
| `UserDefinedURL`  | User Defined URL  | Yes      |

## Binary and specialized frame entries

These entries require format-specific structured data. The ordinary text editor
does not support writing them as freeform strings. A corresponding column may be
empty when the backend does not expose a display value.

| Logical field              | Display label              | Multiple |
| -------------------------- | -------------------------- | -------- |
| `AudioEncodingMethod`      | Audio Encoding Method      | No       |
| `AudioEncryption`          | Audio Encryption           | No       |
| `AudioSeekPointIndex`      | Audio Seek Point Index     | No       |
| `CommercialFrame`          | Commercial Frame           | No       |
| `EncryptionMethod`         | Encryption Method          | No       |
| `EventTimingCodes`         | Event Timing Codes         | No       |
| `GeneralObject`            | General Object             | No       |
| `GroupIdRegistration`      | Group Id Registration      | No       |
| `MusicCDIdentifier`        | Music CD Identifier        | No       |
| `Private`                  | Private                    | No       |
| `RecommendedBufferSize`    | Recommended Buffer Size    | No       |
| `RelativeVolumeAdjustment` | Relative Volume Adjustment | No       |
| `Sequence`                 | Sequence                   | No       |
| `SignatureFrame`           | Signature Frame            | No       |
| `TempoCodes`               | Tempo Codes                | No       |

## More detail

[Metadata editing](/docs/metadata) · [Custom fields](/docs/custom-fields) ·
[Formats and compatibility](/docs/formats)
