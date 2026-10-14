use super::*;

#[derive(Clone, Copy)]
pub struct FreeformSpec {
    pub mean: &'static str,
    pub name: &'static str,
}

pub fn itunes_freeform_spec(key: FrameKey) -> Option<FreeformSpec> {
    use FrameKey::*;
    let mean = "com.apple.iTunes";
    let spec = match key {
        AcoustidId => FreeformSpec {
            mean,
            name: "Acoustid Id",
        },
        AcoustidFingerprint => FreeformSpec {
            mean,
            name: "Acoustid Fingerprint",
        },
        Artists => FreeformSpec {
            mean,
            name: "ARTISTS",
        },
        Asin => FreeformSpec { mean, name: "ASIN" },
        Barcode => FreeformSpec {
            mean,
            name: "BARCODE",
        },
        CatalogNumber => FreeformSpec {
            mean,
            name: "CATALOGNUMBER",
        },
        Compilation => FreeformSpec {
            mean,
            name: "COMPILATION",
        },
        ComposerSort => FreeformSpec {
            mean,
            name: "COMPOSERSORT",
        },
        Conductor => FreeformSpec {
            mean,
            name: "CONDUCTOR",
        },
        InitialKey => FreeformSpec {
            mean,
            name: "initialkey",
        },
        Isrc => FreeformSpec { mean, name: "ISRC" },
        Language => FreeformSpec {
            mean,
            name: "LANGUAGE",
        },
        License => FreeformSpec {
            mean,
            name: "LICENSE",
        },
        Lyricist => FreeformSpec {
            mean,
            name: "LYRICIST",
        },
        SynchronizedLyrics => FreeformSpec {
            mean,
            name: "SYNCEDLYRICS",
        },
        Lyrics => FreeformSpec {
            mean,
            name: "LYRICS",
        },
        Media => FreeformSpec {
            mean,
            name: "MEDIA",
        },
        Mixer => FreeformSpec {
            mean,
            name: "MIXER",
        },
        Mood => FreeformSpec { mean, name: "MOOD" },
        Producer => FreeformSpec {
            mean,
            name: "PRODUCER",
        },
        Rating => FreeformSpec {
            mean,
            name: "RATING",
        },
        ReleaseCountry => FreeformSpec {
            mean,
            name: "MusicBrainz Album Release Country",
        },
        ReleaseStatus => FreeformSpec {
            mean,
            name: "MusicBrainz Album Status",
        },
        ReleaseType => FreeformSpec {
            mean,
            name: "MusicBrainz Album Type",
        },
        Remixer => FreeformSpec {
            mean,
            name: "REMIXER",
        },
        ReplayGainAlbumGain => FreeformSpec {
            mean,
            name: "REPLAYGAIN_ALBUM_GAIN",
        },
        ReplayGainAlbumPeak => FreeformSpec {
            mean,
            name: "REPLAYGAIN_ALBUM_PEAK",
        },
        ReplayGainAlbumRange => FreeformSpec {
            mean,
            name: "REPLAYGAIN_ALBUM_RANGE",
        },
        ReplayGainReferenceLoudness => FreeformSpec {
            mean,
            name: "REPLAYGAIN_REFERENCE_LOUDNESS",
        },
        ReplayGainTrackGain => FreeformSpec {
            mean,
            name: "REPLAYGAIN_TRACK_GAIN",
        },
        ReplayGainTrackPeak => FreeformSpec {
            mean,
            name: "REPLAYGAIN_TRACK_PEAK",
        },
        ReplayGainTrackRange => FreeformSpec {
            mean,
            name: "REPLAYGAIN_TRACK_RANGE",
        },
        Script => FreeformSpec {
            mean,
            name: "SCRIPT",
        },
        Subtitle => FreeformSpec {
            mean,
            name: "SUBTITLE",
        },
        Website => FreeformSpec {
            mean,
            name: "WEBSITE",
        },
        MusicBrainzArtistId => FreeformSpec {
            mean,
            name: "MusicBrainz Artist Id",
        },
        MusicBrainzDiscId => FreeformSpec {
            mean,
            name: "MusicBrainz Disc Id",
        },
        MusicBrainzOriginalArtistId => FreeformSpec {
            mean,
            name: "MusicBrainz Original Artist Id",
        },
        MusicBrainzOriginalAlbumId => FreeformSpec {
            mean,
            name: "MusicBrainz Original Album Id",
        },
        MusicBrainzRecordingId => FreeformSpec {
            mean,
            name: "MusicBrainz Track Id",
        },
        MusicBrainzAlbumArtistId => FreeformSpec {
            mean,
            name: "MusicBrainz Album Artist Id",
        },
        MusicBrainzReleaseGroupId => FreeformSpec {
            mean,
            name: "MusicBrainz Release Group Id",
        },
        MusicBrainzAlbumId => FreeformSpec {
            mean,
            name: "MusicBrainz Album Id",
        },
        MusicBrainzTrackId => FreeformSpec {
            mean,
            name: "MusicBrainz Release Track Id",
        },
        MusicBrainzReleaseTrackId => FreeformSpec {
            mean,
            name: "MusicBrainz Release Track Id",
        },
        MusicBrainzTrmId => FreeformSpec {
            mean,
            name: "MusicBrainz TRM Id",
        },
        MusicBrainzWorkId => FreeformSpec {
            mean,
            name: "MusicBrainz Work Id",
        },
        MusicIpFingerprint => FreeformSpec {
            mean,
            name: "fingerprint",
        },
        MusicIpPuid => FreeformSpec {
            mean,
            name: "MusicIP PUID",
        },
        OriginalAlbum => FreeformSpec {
            mean,
            name: "ORIGINALALBUM",
        },
        OriginalArtist => FreeformSpec {
            mean,
            name: "ORIGINALARTIST",
        },
        OriginalFilename => FreeformSpec {
            mean,
            name: "ORIGINALFILENAME",
        },
        OriginalDate => FreeformSpec {
            mean,
            name: "ORIGINALDATE",
        },
        OriginalYear => FreeformSpec {
            mean,
            name: "ORIGINALYEAR",
        },
        _ => return None,
    };
    Some(spec)
}
