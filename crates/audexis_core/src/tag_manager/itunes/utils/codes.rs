use super::*;

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
        FrameKey::DiscNumber => "disk",
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
