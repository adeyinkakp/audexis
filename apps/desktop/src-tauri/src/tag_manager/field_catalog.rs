use super::{
    traits::Formats,
    utils::{FrameKey, FrameKeyKind},
};
use serde::Serialize;

pub const FIELDS: &[(FrameKey, &str)] = &[
    (FrameKey::Title, "Title"),
    (FrameKey::Artist, "Artist"),
    (FrameKey::Album, "Album"),
    (FrameKey::Year, "Year"),
    (FrameKey::TrackNumber, "Track Number"),
    (FrameKey::Genre, "Genre"),
    (FrameKey::AlbumArtist, "Album Artist"),
    (FrameKey::AcoustidId, "AcoustID Id"),
    (FrameKey::AcoustidFingerprint, "AcoustID Fingerprint"),
    (FrameKey::AlbumArtistSort, "Album Artist Sort"),
    (FrameKey::AlbumSort, "Album Sort"),
    (FrameKey::Arranger, "Arranger"),
    (FrameKey::ArtistSort, "Artist Sort"),
    (FrameKey::Artists, "Artists"),
    (FrameKey::Asin, "Asin"),
    (FrameKey::Barcode, "Barcode"),
    (FrameKey::CatalogNumber, "Catalog Number"),
    (FrameKey::Compilation, "Compilation"),
    (FrameKey::ComposerSort, "Composer Sort"),
    (FrameKey::Director, "Director"),
    (FrameKey::DiscNumber, "Disc Number"),
    (FrameKey::DiscSubtitle, "Disc Subtitle"),
    (FrameKey::EncoderSettings, "Encoder Settings"),
    (FrameKey::Engineer, "Engineer"),
    (FrameKey::Gapless, "Gapless"),
    (FrameKey::Grouping, "Grouping"),
    (FrameKey::InitialKey, "Initial Key"),
    (FrameKey::Isrc, "Isrc"),
    (FrameKey::License, "License"),
    (FrameKey::Lyricist, "Lyricist"),
    (FrameKey::Lyrics, "Lyrics"),
    (FrameKey::Media, "Media"),
    (FrameKey::Mixer, "Mixer"),
    (FrameKey::Mood, "Mood"),
    (FrameKey::Movement, "Movement"),
    (FrameKey::MovementTotal, "Movement Total"),
    (FrameKey::MovementNumber, "Movement Number"),
    (FrameKey::MusicBrainzArtistId, "MusicBrainz Artist Id"),
    (FrameKey::MusicBrainzDiscId, "MusicBrainz Disc Id"),
    (
        FrameKey::MusicBrainzOriginalArtistId,
        "MusicBrainz Original Artist Id",
    ),
    (
        FrameKey::MusicBrainzOriginalAlbumId,
        "MusicBrainz Original Album Id",
    ),
    (FrameKey::MusicBrainzRecordingId, "MusicBrainz Recording Id"),
    (
        FrameKey::MusicBrainzAlbumArtistId,
        "MusicBrainz Album Artist Id",
    ),
    (
        FrameKey::MusicBrainzReleaseGroupId,
        "MusicBrainz Release Group Id",
    ),
    (FrameKey::MusicBrainzAlbumId, "MusicBrainz Album Id"),
    (FrameKey::MusicBrainzTrackId, "MusicBrainz Track Id"),
    (
        FrameKey::MusicBrainzReleaseTrackId,
        "MusicBrainz Release Track Id",
    ),
    (FrameKey::MusicBrainzTrmId, "MusicBrainz Trm Id"),
    (FrameKey::MusicBrainzWorkId, "MusicBrainz Work Id"),
    (FrameKey::MusicIpFingerprint, "Music Ip Fingerprint"),
    (FrameKey::MusicIpPuid, "Music Ip Puid"),
    (FrameKey::OriginalAlbum, "Original Album"),
    (FrameKey::OriginalArtist, "Original Artist"),
    (FrameKey::OriginalFilename, "Original Filename"),
    (FrameKey::OriginalDate, "Original Date"),
    (FrameKey::OriginalYear, "Original Year"),
    (FrameKey::Performer, "Performer"),
    (FrameKey::Podcast, "Podcast"),
    (FrameKey::PodcastUrl, "Podcast Url"),
    (FrameKey::Producer, "Producer"),
    (FrameKey::Rating, "Rating"),
    (FrameKey::Label, "Label"),
    (FrameKey::ReleaseCountry, "Release Country"),
    (FrameKey::ReleaseStatus, "Release Status"),
    (FrameKey::ReleaseType, "Release Type"),
    (FrameKey::Remixer, "Remixer"),
    (FrameKey::ReplayGainAlbumGain, "Replay Gain Album Gain"),
    (FrameKey::ReplayGainAlbumPeak, "Replay Gain Album Peak"),
    (FrameKey::ReplayGainAlbumRange, "Replay Gain Album Range"),
    (
        FrameKey::ReplayGainReferenceLoudness,
        "Replay Gain Reference Loudness",
    ),
    (FrameKey::ReplayGainTrackGain, "Replay Gain Track Gain"),
    (FrameKey::ReplayGainTrackPeak, "Replay Gain Track Peak"),
    (FrameKey::ReplayGainTrackRange, "Replay Gain Track Range"),
    (FrameKey::Script, "Script"),
    (FrameKey::Show, "Show"),
    (FrameKey::ShowSort, "Show Sort"),
    (FrameKey::ShowMovement, "Show Movement"),
    (FrameKey::Subtitle, "Subtitle"),
    (FrameKey::TotalDiscs, "Total Discs"),
    (FrameKey::TotalTracks, "Total Tracks"),
    (FrameKey::TitleSort, "Title Sort"),
    (FrameKey::Website, "Website"),
    (FrameKey::Work, "Work"),
    (FrameKey::Writer, "Writer"),
    (FrameKey::ContentGroup, "Content Group"),
    (FrameKey::Composer, "Composer"),
    (FrameKey::EncodedBy, "Encoded By"),
    (FrameKey::UnsyncedLyrics, "Unsynced Lyrics"),
    (FrameKey::Length, "Length"),
    (FrameKey::Conductor, "Conductor"),
    (FrameKey::AttachedPicture, "Attached Picture"),
    (FrameKey::UserDefinedURL, "User Defined URL"),
    (FrameKey::Comments, "Comments"),
    (FrameKey::Private, "Private"),
    (
        FrameKey::RelativeVolumeAdjustment,
        "Relative Volume Adjustment",
    ),
    (FrameKey::EncryptionMethod, "Encryption Method"),
    (FrameKey::GroupIdRegistration, "Group Id Registration"),
    (FrameKey::GeneralObject, "General Object"),
    (FrameKey::CommercialURL, "Commercial URL"),
    (FrameKey::CopyrightURL, "Copyright URL"),
    (FrameKey::AudioFileURL, "Audio File URL"),
    (FrameKey::ArtistURL, "Artist URL"),
    (FrameKey::RadioStationURL, "Radio Station URL"),
    (FrameKey::PaymentURL, "Payment URL"),
    (FrameKey::BitmapImageURL, "Bitmap Image URL"),
    (FrameKey::UserDefinedText, "User Defined Text"),
    (FrameKey::SynchronizedLyrics, "Synchronized Lyrics"),
    (FrameKey::TempoCodes, "Tempo Codes"),
    (FrameKey::MusicCDIdentifier, "Music CD Identifier"),
    (FrameKey::EventTimingCodes, "Event Timing Codes"),
    (FrameKey::Sequence, "Sequence"),
    (FrameKey::PlayCount, "Play Count"),
    (FrameKey::AudioSeekPointIndex, "Audio Seek Point Index"),
    (FrameKey::MediaType, "Media Type"),
    (FrameKey::CommercialFrame, "Commercial Frame"),
    (FrameKey::AudioEncryption, "Audio Encryption"),
    (FrameKey::SignatureFrame, "Signature Frame"),
    (FrameKey::SoftwareEncoder, "Software Encoder"),
    (FrameKey::AudioEncodingMethod, "Audio Encoding Method"),
    (FrameKey::RecommendedBufferSize, "Recommended Buffer Size"),
    (FrameKey::BeatsPerMinute, "Beats Per Minute"),
    (FrameKey::Language, "Language"),
    (FrameKey::FileType, "File Type"),
    (FrameKey::Time, "Time"),
    (FrameKey::RecordingDate, "Recording Date"),
    (FrameKey::ReleaseDate, "Release Date"),
];

pub fn is_text(key: FrameKey) -> bool {
    !matches!(
        key,
        FrameKey::AttachedPicture
            | FrameKey::UserDefinedText
            | FrameKey::UserDefinedURL
            | FrameKey::Private
            | FrameKey::RelativeVolumeAdjustment
            | FrameKey::EncryptionMethod
            | FrameKey::GroupIdRegistration
            | FrameKey::GeneralObject
            | FrameKey::TempoCodes
            | FrameKey::MusicCDIdentifier
            | FrameKey::EventTimingCodes
            | FrameKey::Sequence
            | FrameKey::AudioSeekPointIndex
            | FrameKey::CommercialFrame
            | FrameKey::AudioEncryption
            | FrameKey::SignatureFrame
            | FrameKey::AudioEncodingMethod
            | FrameKey::RecommendedBufferSize
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldInfo {
    pub key: FrameKey,
    pub storage_key: String,
    pub label: &'static str,
    pub kind: &'static str,
    pub multi_value: bool,
    pub editable: bool,
    pub native_key: Option<String>,
}

pub fn catalog(format: Option<&Formats>) -> Vec<FieldInfo> {
    FIELDS
        .iter()
        .map(|&(key, label)| FieldInfo {
            key,
            label,
            storage_key: key.to_string(),
            kind: if matches!(key, FrameKey::UserDefinedText | FrameKey::UserDefinedURL) {
                "custom"
            } else if key == FrameKey::AttachedPicture {
                "artwork"
            } else if !is_text(key) {
                "binary"
            } else if matches!(key.get_kind(), FrameKeyKind::URL) {
                "url"
            } else {
                "text"
            },
            multi_value: key.is_multi_valued(),
            editable: is_text(key)
                && format.is_none_or(|format| {
                    super::custom_fields::supported(format)
                        || (matches!(format, Formats::Id3v10 | Formats::Id3v11)
                            && matches!(
                                key,
                                FrameKey::Title
                                    | FrameKey::Artist
                                    | FrameKey::Album
                                    | FrameKey::Year
                                    | FrameKey::Genre
                                    | FrameKey::Comments
                                    | FrameKey::TrackNumber
                            ))
                }),
            native_key: format
                .and_then(|format| super::portable_fields::fallback_name(format, key)),
        })
        .collect()
}
