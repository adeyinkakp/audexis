use super::*;

pub static FREEFORM_REVERSE_MAP: Lazy<HashMap<(&'static str, &'static str), FrameKey>> =
    Lazy::new(|| {
        let mut map = HashMap::new();
        let entries: &[(FreeformSpec, FrameKey)] = &[
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "Acoustid Id",
                },
                FrameKey::AcoustidId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "Acoustid Fingerprint",
                },
                FrameKey::AcoustidFingerprint,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "ARTISTS",
                },
                FrameKey::Artists,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "ASIN",
                },
                FrameKey::Asin,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "BARCODE",
                },
                FrameKey::Barcode,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "CATALOGNUMBER",
                },
                FrameKey::CatalogNumber,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "initialkey",
                },
                FrameKey::InitialKey,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "ISRC",
                },
                FrameKey::Isrc,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "LICENSE",
                },
                FrameKey::License,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "LYRICIST",
                },
                FrameKey::Lyricist,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "LYRICS",
                },
                FrameKey::Lyrics,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "SYNCEDLYRICS",
                },
                FrameKey::SynchronizedLyrics,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MEDIA",
                },
                FrameKey::Media,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MIXER",
                },
                FrameKey::Mixer,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MOOD",
                },
                FrameKey::Mood,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "PRODUCER",
                },
                FrameKey::Producer,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Album Release Country",
                },
                FrameKey::ReleaseCountry,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Album Status",
                },
                FrameKey::ReleaseStatus,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Album Type",
                },
                FrameKey::ReleaseType,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "REMIXER",
                },
                FrameKey::Remixer,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "REPLAYGAIN_ALBUM_GAIN",
                },
                FrameKey::ReplayGainAlbumGain,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "REPLAYGAIN_ALBUM_PEAK",
                },
                FrameKey::ReplayGainAlbumPeak,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "REPLAYGAIN_ALBUM_RANGE",
                },
                FrameKey::ReplayGainAlbumRange,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "REPLAYGAIN_REFERENCE_LOUDNESS",
                },
                FrameKey::ReplayGainReferenceLoudness,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "REPLAYGAIN_TRACK_GAIN",
                },
                FrameKey::ReplayGainTrackGain,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "REPLAYGAIN_TRACK_PEAK",
                },
                FrameKey::ReplayGainTrackPeak,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "REPLAYGAIN_TRACK_RANGE",
                },
                FrameKey::ReplayGainTrackRange,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "SCRIPT",
                },
                FrameKey::Script,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "SUBTITLE",
                },
                FrameKey::Subtitle,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "WEBSITE",
                },
                FrameKey::Website,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Artist Id",
                },
                FrameKey::MusicBrainzArtistId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Disc Id",
                },
                FrameKey::MusicBrainzDiscId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Original Artist Id",
                },
                FrameKey::MusicBrainzOriginalArtistId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Original Album Id",
                },
                FrameKey::MusicBrainzOriginalAlbumId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Track Id",
                },
                FrameKey::MusicBrainzRecordingId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Album Artist Id",
                },
                FrameKey::MusicBrainzAlbumArtistId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Release Group Id",
                },
                FrameKey::MusicBrainzReleaseGroupId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Album Id",
                },
                FrameKey::MusicBrainzAlbumId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Release Track Id",
                },
                FrameKey::MusicBrainzTrackId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Release Track Id",
                },
                FrameKey::MusicBrainzReleaseTrackId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz TRM Id",
                },
                FrameKey::MusicBrainzTrmId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicBrainz Work Id",
                },
                FrameKey::MusicBrainzWorkId,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "fingerprint",
                },
                FrameKey::MusicIpFingerprint,
            ),
            (
                FreeformSpec {
                    mean: "com.apple.iTunes",
                    name: "MusicIP PUID",
                },
                FrameKey::MusicIpPuid,
            ),
        ];
        for (spec, key) in entries {
            map.insert((spec.mean, spec.name), *key);
        }
        map
    });
