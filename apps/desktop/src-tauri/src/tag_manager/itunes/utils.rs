use crate::tag_manager;
use crate::tag_manager::utils::FrameKey;
use crate::tag_manager::utils::TagValue;
use once_cell::sync::Lazy;
use std::collections::HashMap;

pub fn itunes_code(key: FrameKey) -> &'static str {
    match key {
        FrameKey::Title => "©nam",
        FrameKey::Artist => "©ART",
        FrameKey::Album => "©alb",
        FrameKey::AlbumArtist => "aART",
        FrameKey::Composer => "©wrt",
        FrameKey::ContentGroup => "©grp",
        FrameKey::Genre => "©gen",
        FrameKey::TrackNumber => "trkn",
        FrameKey::BeatsPerMinute => "tmpo",
        FrameKey::AttachedPicture => "covr",
        FrameKey::Comments => "©cmt",
        FrameKey::UnsyncedLyrics => "©lyr",
        FrameKey::EncodedBy => "©too",
        FrameKey::SoftwareEncoder => "©too",
        FrameKey::CopyrightURL => "cprt",
        FrameKey::MediaType => "stik",
        FrameKey::PlayCount => "pcnt",
        FrameKey::Length => "©len",

        FrameKey::Year => "©day",
        FrameKey::RecordingDate => "©day",
        FrameKey::ReleaseDate => "©day",

        FrameKey::UserDefinedText => "----",
        FrameKey::UserDefinedURL => "----",
        FrameKey::Private => "----",
        FrameKey::RelativeVolumeAdjustment => "----",
        FrameKey::EncryptionMethod => "----",
        FrameKey::GroupIdRegistration => "----",
        FrameKey::GeneralObject => "----",
        FrameKey::CommercialURL => "----",
        FrameKey::AudioFileURL => "----",
        FrameKey::ArtistURL => "----",
        FrameKey::RadioStationURL => "----",
        FrameKey::PaymentURL => "----",
        FrameKey::BitmapImageURL => "----",
        FrameKey::SynchronizedLyrics => "----",
        FrameKey::TempoCodes => "----",
        FrameKey::MusicCDIdentifier => "----",
        FrameKey::EventTimingCodes => "----",
        FrameKey::Sequence => "----",
        FrameKey::AudioSeekPointIndex => "----",
        FrameKey::CommercialFrame => "----",
        FrameKey::AudioEncryption => "----",
        FrameKey::SignatureFrame => "----",
        FrameKey::AudioEncodingMethod => "----",
        FrameKey::RecommendedBufferSize => "----",
        FrameKey::Language => "----",
        FrameKey::FileType => "----",
        FrameKey::Time => "----",

        _ => "----",
    }
}

pub fn itunes_key(code: &str) -> Option<FrameKey> {
    match code {
        "©nam" => Some(FrameKey::Title),
        "©ART" => Some(FrameKey::Artist),
        "©alb" => Some(FrameKey::Album),
        "aART" => Some(FrameKey::AlbumArtist),
        "©wrt" => Some(FrameKey::Composer),
        "©grp" => Some(FrameKey::ContentGroup),
        "©gen" => Some(FrameKey::Genre),
        "trkn" => Some(FrameKey::TrackNumber),
        "tmpo" => Some(FrameKey::BeatsPerMinute),
        "covr" => Some(FrameKey::AttachedPicture),
        "©cmt" => Some(FrameKey::Comments),
        "©lyr" => Some(FrameKey::UnsyncedLyrics),
        "©too" => Some(FrameKey::EncodedBy),
        "cprt" => Some(FrameKey::CopyrightURL),
        "stik" => Some(FrameKey::MediaType),
        "pcnt" => Some(FrameKey::PlayCount),
        "©len" => Some(FrameKey::Length),

        "©day" => Some(FrameKey::RecordingDate),

        "----" => Some(FrameKey::UserDefinedText),

        _ => None,
    }
}
#[derive(Debug, Clone)]
pub struct AtomFlag {
    pub name: &'static str,
    pub flag: [u8; 4],
    pub size: Option<u32>,
    #[allow(dead_code)]
    pub boolean: bool,
    #[allow(dead_code)]
    pub no_size_limit: bool,
}

pub static ATOMS_WITH_FLAGS: &[AtomFlag] = &[
    AtomFlag {
        name: "©alb",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "©ART",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "aART",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "©cmt",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "©day",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "©nam",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "©gen",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "gnre",
        flag: [0x00, 0x00, 0x00, 0x00],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "trkn",
        flag: [0x00, 0x00, 0x00, 0x00],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "disk",
        flag: [0x00, 0x00, 0x00, 0x00],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "©wrt",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "©too",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "tmpo",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "cprt",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "cpil",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: None,
        boolean: true,
        no_size_limit: false,
    },
    AtomFlag {
        name: "covr",
        flag: [0x00, 0x00, 0x00, 0x0e],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "rtng",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "©grp",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "stik",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "pcst",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: None,
        boolean: true,
        no_size_limit: false,
    },
    AtomFlag {
        name: "catg",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "keyw",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "purl",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "egid",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "desc",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: true,
    },
    AtomFlag {
        name: "©lyr",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: true,
    },
    AtomFlag {
        name: "tvnn",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "tvsh",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "tven",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "tvsn",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: Some(32),
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "tves",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: Some(32),
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "purd",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "apID",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "pgap",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: None,
        boolean: true,
        no_size_limit: false,
    },
    AtomFlag {
        name: "akID",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "cnID",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: Some(32),
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "atID",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: Some(32),
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "sfID",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: Some(32),
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "geID",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: Some(32),
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "plID",
        flag: [0x00, 0x00, 0x00, 0x15],
        size: Some(64),
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "soal",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "soaa",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "soar",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "sonm",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "sosn",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "soco",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "©wrk",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "©mvn",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "mvi",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
    AtomFlag {
        name: "mvc",
        flag: [0x00, 0x00, 0x00, 0x01],
        size: None,
        boolean: false,
        no_size_limit: false,
    },
];

pub fn get_atom_flag(name: &str) -> Option<&'static AtomFlag> {
    ATOMS_WITH_FLAGS.iter().find(|atom| atom.name == name)
}

pub static ITUNES_REVERSE_MAP: Lazy<HashMap<&'static str, FrameKey>> = Lazy::new(|| {
    let mut map = HashMap::new();

    let mappings = [
        ("©nam", FrameKey::Title),
        ("©ART", FrameKey::Artist),
        ("©alb", FrameKey::Album),
        ("©day", FrameKey::Year),
        ("trkn", FrameKey::TrackNumber),
        ("©gen", FrameKey::Genre),
        // ("gnre", FrameKey::Genre), // Alternative genre code (numeric)
        ("aART", FrameKey::AlbumArtist),
        ("©grp", FrameKey::ContentGroup),
        ("©wrt", FrameKey::Composer),
        ("©too", FrameKey::EncodedBy),
        ("©lyr", FrameKey::UnsyncedLyrics),
        ("covr", FrameKey::AttachedPicture),
        ("©cmt", FrameKey::Comments),
        ("cprt", FrameKey::CopyrightURL),
        ("pcnt", FrameKey::PlayCount),
        ("stik", FrameKey::MediaType),
        ("tmpo", FrameKey::BeatsPerMinute),
        ("disk", FrameKey::DiscNumber), // Disk number (similar to track)
        ("©enc", FrameKey::EncodedBy),  // Encoded by (alternative)
        ("soal", FrameKey::AlbumSort),  // Sort album
        ("soaa", FrameKey::AlbumArtistSort), // Sort album artist
        ("soar", FrameKey::ArtistSort), // Sort artist
        ("sonm", FrameKey::TitleSort),  // Sort name
        ("soco", FrameKey::Composer),   // Sort composer
        ("rtng", FrameKey::MediaType),  // Rating/Content rating
        ("cpil", FrameKey::MediaType),  // Compilation flag
        ("pgap", FrameKey::MediaType),  // Gapless playback
        ("©des", FrameKey::Comments),   // Description
        // ("ldes", FrameKey::Comments),       // Long description
        // ("©pub", FrameKey::EncodedBy),      // Publisher
        // ("purd", FrameKey::Year),           // Purchase date
        ("catg", FrameKey::Genre),        // Category (podcast)
        ("keyw", FrameKey::ContentGroup), // Keywords
        // ("egid", FrameKey::UserDefinedURL), // Episode global unique ID
        // ("©mvn", FrameKey::Title),       // Movement name
        // ("©mvi", FrameKey::TrackNumber), // Movement number
        // ("©mvc", FrameKey::TrackNumber), // Movement count
        ("purl", FrameKey::PodcastUrl), // Podcast URL
        // ("tven", FrameKey::Title),       // TV episode name
        ("tvsh", FrameKey::Show), // Show name
        ("pcst", FrameKey::Podcast),
        ("©mvn", FrameKey::Movement),
        ("mvi", FrameKey::MovementNumber),
        ("mvc", FrameKey::MovementTotal),
        ("©wrk", FrameKey::Work),
        // ("tves", FrameKey::TrackNumber), // TV episode number
        // ("tvsn", FrameKey::TrackNumber), // TV season
        // ("tvnn", FrameKey::Title),       // TV network name
    ];

    for (code, key) in mappings {
        map.insert(code, key);
    }

    map
});

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

fn split_values(text: &str) -> impl Iterator<Item = &str> {
    if text.contains('\u{0}') {
        text.split('\u{0}')
    } else if text.contains(';') {
        text.split(';')
    } else {
        text.split('/')
    }
}

pub fn raw_to_tags(raw: &[(String, TagValue)]) -> HashMap<FrameKey, Vec<TagValue>> {
    let mut result: HashMap<FrameKey, Vec<TagValue>> = HashMap::new();

    for (k, v) in raw.iter() {
        if let Some(&key) = ITUNES_REVERSE_MAP.get(k.as_str()) {
            match v {
                TagValue::Text(s)
                    if matches!(key, FrameKey::Artist | FrameKey::Genre)
                        && (s.contains('/') || s.contains('\u{0}')) =>
                {
                    for part in split_values(s) {
                        let seg = part.trim();
                        if !seg.is_empty() {
                            result
                                .entry(key)
                                .or_default()
                                .push(TagValue::Text(seg.to_string()));
                        }
                    }
                }
                _ => result.entry(key).or_default().push(v.clone()),
            }
            continue;
        }
        if let Some(rest) = k.strip_prefix("----:") {
            if let Some((mean, name)) = rest.split_once(':') {
                if let Some(&fk) = FREEFORM_REVERSE_MAP.get(&(mean, name)) {
                    result.entry(fk).or_default().push(v.clone());
                    continue;
                }
                if mean == "com.apple.iTunes" {
                    use FrameKey::UserDefinedText;
                    if let TagValue::Text(s) = v {
                        for part in split_values(s) {
                            let seg = part.trim();
                            if seg.is_empty() {
                                continue;
                            }
                            result
                                .entry(UserDefinedText)
                                .or_default()
                                .push(TagValue::UserText(tag_manager::utils::UserTextEntry {
                                    description: name.to_string(),
                                    value: seg.to_string(),
                                }));
                        }
                    }
                    continue;
                }
            }
        }
    }

    result
}
