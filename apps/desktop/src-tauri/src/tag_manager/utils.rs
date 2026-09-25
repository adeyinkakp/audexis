use crate::tag_manager::traits::Formats;

use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum FrameKeyKind {
    Image,
    Text,
    URL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FrameKey {
    Title,
    Artist,
    Album,
    Year,
    TrackNumber,
    Genre,
    AlbumArtist,
    AcoustidId,
    AcoustidFingerprint,
    AlbumArtistSort,
    AlbumSort,
    Arranger,
    ArtistSort,
    Artists,
    Asin,
    Barcode,
    CatalogNumber,
    Compilation,
    ComposerSort,
    Director,
    DiscNumber,
    DiscSubtitle,
    EncoderSettings,
    Engineer,
    Gapless,
    Grouping,
    InitialKey,
    Isrc,
    License,
    Lyricist,
    Lyrics,
    Media,
    Mixer,
    Mood,
    Movement,
    MovementTotal,
    MovementNumber,
    MusicBrainzArtistId,
    MusicBrainzDiscId,
    MusicBrainzOriginalArtistId,
    MusicBrainzOriginalAlbumId,
    MusicBrainzRecordingId,
    MusicBrainzAlbumArtistId,
    MusicBrainzReleaseGroupId,
    MusicBrainzAlbumId,
    MusicBrainzTrackId,
    MusicBrainzReleaseTrackId,
    MusicBrainzTrmId,
    MusicBrainzWorkId,
    MusicIpFingerprint,
    MusicIpPuid,
    OriginalAlbum,
    OriginalArtist,
    OriginalFilename,
    OriginalDate,
    OriginalYear,
    Performer,
    Podcast,
    PodcastUrl,
    Producer,
    Rating,
    Label,
    ReleaseCountry,
    ReleaseStatus,
    ReleaseType,
    Remixer,
    ReplayGainAlbumGain,
    ReplayGainAlbumPeak,
    ReplayGainAlbumRange,
    ReplayGainReferenceLoudness,
    ReplayGainTrackGain,
    ReplayGainTrackPeak,
    ReplayGainTrackRange,
    Script,
    Show,
    ShowSort,
    ShowMovement,
    Subtitle,
    TotalDiscs,
    TotalTracks,
    TitleSort,
    Website,
    Work,
    Writer,
    ContentGroup,
    Composer,
    EncodedBy,
    UnsyncedLyrics,
    Length,
    Conductor,
    AttachedPicture,
    UserDefinedURL,
    Comments,
    Private,
    RelativeVolumeAdjustment,
    EncryptionMethod,
    GroupIdRegistration,
    GeneralObject,
    CommercialURL,
    CopyrightURL,
    AudioFileURL,
    ArtistURL,
    RadioStationURL,
    PaymentURL,
    BitmapImageURL,
    UserDefinedText,
    SynchronizedLyrics,
    TempoCodes,
    MusicCDIdentifier,
    EventTimingCodes,
    Sequence,
    PlayCount,
    AudioSeekPointIndex,
    MediaType,
    CommercialFrame,
    AudioEncryption,
    SignatureFrame,
    SoftwareEncoder,
    AudioEncodingMethod,
    RecommendedBufferSize,
    BeatsPerMinute,
    Language,
    FileType,
    Time,
    RecordingDate,
    ReleaseDate,
}
impl FrameKey {
    pub fn is_multi_valued(&self) -> bool {
        match self {
            FrameKey::AttachedPicture
            | FrameKey::UserDefinedText
            | FrameKey::UserDefinedURL
            | FrameKey::Genre
            | FrameKey::Artist
            | FrameKey::AlbumArtist
            | FrameKey::Composer
            | FrameKey::Lyricist
            | FrameKey::Comments => true,
            _ => false,
        }
    }
    pub fn get_kind(&self) -> FrameKeyKind {
        match self {
            FrameKey::AttachedPicture => FrameKeyKind::Image,

            FrameKey::PodcastUrl
            | FrameKey::Website
            | FrameKey::UserDefinedURL
            | FrameKey::CommercialURL
            | FrameKey::CopyrightURL
            | FrameKey::AudioFileURL
            | FrameKey::ArtistURL
            | FrameKey::RadioStationURL
            | FrameKey::PaymentURL
            | FrameKey::BitmapImageURL => FrameKeyKind::URL,

            _ => FrameKeyKind::Text,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TagValue {
    Text(String),
    Picture {
        mime: String,
        data: Vec<u8>,
        picture_type: Option<u8>,
        description: Option<String>,
    },
    UserText(UserTextEntry),
    UserUrl(UserUrlEntry),
    Comment {
        encoding: String,
        language: String,
        description: String,
        text: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PictureData {
    pub mime: String,
    pub data: Vec<u8>,
    pub picture_type: Option<u8>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserTextEntry {
    pub description: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserUrlEntry {
    pub description: String,
    pub url: String,
}

pub type TagMap = HashMap<String, Vec<TagValue>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FreeformTag {
    pub mean: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
/// File struct that can be sent to frontend via ipc
pub struct SerializableFile {
    pub path: String,
    pub file_name: String,
    pub tag_format: String,
    pub tag_formats: Vec<String>,
    pub tags: HashMap<String, Vec<SerializableTagValue>>,
    pub freeforms: Vec<SerializableFreeform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", content = "values", rename_all = "camelCase")]
pub enum TagChange {
    Replace(Vec<SerializableTagValue>),
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Changes {
    pub paths: Vec<String>,
    pub tags: HashMap<FrameKey, TagChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableTagFrame {
    pub key: FrameKey,
    pub values: Vec<SerializableTagValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameChanges {
    pub paths: Vec<String>,
    pub frames: Vec<SerializableTagFrame>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SerializableTagValue {
    Text(String),
    Picture {
        mime: String,
        data_base64: String,
        picture_type: Option<u8>,
        description: Option<String>,
    },
    UserText(UserTextEntry),
    UserUrl(UserUrlEntry),
    Comment {
        encoding: String,
        language: String,
        description: String,
        text: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupRule {
    ReplaceUnderscores,
    NormalizeDashes,
    FixCapitalization,
    TrimWhitespace,
    CollapseSpaces,

    RemoveSuffixes,
    NormalizeFeat,
    RemoveBrackets,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableFreeform {
    pub mean: String,
    pub name: String,
    pub value: String,
}

/// Internal File Struct
#[derive(Debug, Clone)]
pub struct MetadataFile {
    pub path: PathBuf,

    pub tags: HashMap<FrameKey, Vec<TagValue>>,
    pub tag_format: Formats,
    pub tag_formats: Vec<Formats>,
    pub freeforms: Vec<FreeformTag>,
}

impl fmt::Display for TagValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TagValue::Text(s) => write!(f, "{}", s),
            TagValue::Picture {
                mime,
                data,
                picture_type,
                description,
            } => {
                let t = picture_type
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".to_string());
                let desc = description.clone().unwrap_or_default();
                write!(
                    f,
                    "Picture (type {}, desc '{}', MIME: {}, {} bytes)",
                    t,
                    desc,
                    mime,
                    data.len()
                )
            }
            TagValue::UserText(ut) => write!(f, "{}={}", ut.description, ut.value),
            TagValue::UserUrl(uu) => write!(f, "{}={}", uu.description, uu.url),
            TagValue::Comment {
                encoding,
                language,
                description,
                text,
            } => write!(
                f,
                "Comment (encoding: {}, language: {}, description: {}, text: {})",
                encoding, language, description, text
            ),
        }
    }
}

/// FrameKey to string

impl fmt::Display for FrameKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FrameKey::Title => "title",
            FrameKey::Artist => "artist",
            FrameKey::Album => "album",
            FrameKey::Year => "year",
            FrameKey::TrackNumber => "trackNumber",
            FrameKey::Genre => "genre",
            FrameKey::AlbumArtist => "albumArtist",
            FrameKey::AcoustidId => "acoustid_id",
            FrameKey::AcoustidFingerprint => "acoustid_fingerprint",
            FrameKey::AlbumArtistSort => "albumartistsort",
            FrameKey::AlbumSort => "albumsort",
            FrameKey::Arranger => "arranger",
            FrameKey::ArtistSort => "artistsort",
            FrameKey::Artists => "artists",
            FrameKey::Asin => "asin",
            FrameKey::Barcode => "barcode",
            FrameKey::CatalogNumber => "catalognumber",
            FrameKey::Compilation => "compilation",
            FrameKey::ComposerSort => "composersort",
            FrameKey::Director => "director",
            FrameKey::DiscNumber => "discnumber",
            FrameKey::DiscSubtitle => "discsubtitle",
            FrameKey::EncoderSettings => "encodersettings",
            FrameKey::Engineer => "engineer",
            FrameKey::Gapless => "gapless",
            FrameKey::Grouping => "grouping",
            FrameKey::InitialKey => "key",
            FrameKey::Isrc => "isrc",
            FrameKey::License => "license",
            FrameKey::Lyricist => "lyricist",
            FrameKey::Lyrics => "lyrics",
            FrameKey::Media => "media",
            FrameKey::Mixer => "mixer",
            FrameKey::Mood => "mood",
            FrameKey::Movement => "movement",
            FrameKey::MovementTotal => "movementtotal",
            FrameKey::MovementNumber => "movementnumber",
            FrameKey::MusicBrainzArtistId => "musicbrainz_artistid",
            FrameKey::MusicBrainzDiscId => "musicbrainz_discid",
            FrameKey::MusicBrainzOriginalArtistId => "musicbrainz_originalartistid",
            FrameKey::MusicBrainzOriginalAlbumId => "musicbrainz_originalalbumid",
            FrameKey::MusicBrainzRecordingId => "musicbrainz_recordingid",
            FrameKey::MusicBrainzAlbumArtistId => "musicbrainz_albumartistid",
            FrameKey::MusicBrainzReleaseGroupId => "musicbrainz_releasegroupid",
            FrameKey::MusicBrainzAlbumId => "musicbrainz_albumid",
            FrameKey::MusicBrainzTrackId => "musicbrainz_trackid",
            FrameKey::MusicBrainzReleaseTrackId => "musicbrainz_releasetrackid",
            FrameKey::MusicBrainzTrmId => "musicbrainz_trmid",
            FrameKey::MusicBrainzWorkId => "musicbrainz_workid",
            FrameKey::MusicIpFingerprint => "musicip_fingerprint",
            FrameKey::MusicIpPuid => "musicip_puid",
            FrameKey::OriginalAlbum => "originalalbum",
            FrameKey::OriginalArtist => "originalartist",
            FrameKey::OriginalFilename => "originalfilename",
            FrameKey::OriginalDate => "originaldate",
            FrameKey::OriginalYear => "originalyear",
            FrameKey::Performer => "performer",
            FrameKey::Podcast => "podcast",
            FrameKey::PodcastUrl => "podcasturl",
            FrameKey::Producer => "producer",
            FrameKey::Rating => "_rating",
            FrameKey::Label => "label",
            FrameKey::ReleaseCountry => "releasecountry",
            FrameKey::ReleaseStatus => "releasestatus",
            FrameKey::ReleaseType => "releasetype",
            FrameKey::Remixer => "remixer",
            FrameKey::ReplayGainAlbumGain => "replaygain_album_gain",
            FrameKey::ReplayGainAlbumPeak => "replaygain_album_peak",
            FrameKey::ReplayGainAlbumRange => "replaygain_album_range",
            FrameKey::ReplayGainReferenceLoudness => "replaygain_reference_loudness",
            FrameKey::ReplayGainTrackGain => "replaygain_track_gain",
            FrameKey::ReplayGainTrackPeak => "replaygain_track_peak",
            FrameKey::ReplayGainTrackRange => "replaygain_track_range",
            FrameKey::Script => "script",
            FrameKey::Show => "show",
            FrameKey::ShowSort => "showsort",
            FrameKey::ShowMovement => "showmovement",
            FrameKey::Subtitle => "subtitle",
            FrameKey::TotalDiscs => "totaldiscs",
            FrameKey::TotalTracks => "totaltracks",
            FrameKey::TitleSort => "titlesort",
            FrameKey::Website => "website",
            FrameKey::Work => "work",
            FrameKey::Writer => "writer",
            FrameKey::ContentGroup => "contentGroup",
            FrameKey::Composer => "composer",
            FrameKey::EncodedBy => "encodedBy",
            FrameKey::UnsyncedLyrics => "unsyncedLyrics",
            FrameKey::Length => "length",
            FrameKey::Conductor => "conductor",
            FrameKey::AttachedPicture => "attachedPicture",
            FrameKey::UserDefinedURL => "userDefinedUrl",
            FrameKey::Comments => "comments",
            FrameKey::Private => "private",
            FrameKey::RelativeVolumeAdjustment => "relativeVolumeAdjustment",
            FrameKey::EncryptionMethod => "encryptionMethod",
            FrameKey::GroupIdRegistration => "groupIdRegistration",
            FrameKey::GeneralObject => "generalObject",
            FrameKey::CommercialURL => "commercialUrl",
            FrameKey::CopyrightURL => "copyrightUrl",
            FrameKey::AudioFileURL => "audioFileUrl",
            FrameKey::ArtistURL => "artistUrl",
            FrameKey::RadioStationURL => "radioStationUrl",
            FrameKey::PaymentURL => "paymentUrl",
            FrameKey::BitmapImageURL => "bitmapImageUrl",
            FrameKey::UserDefinedText => "userDefinedText",
            FrameKey::SynchronizedLyrics => "synchronizedLyrics",
            FrameKey::TempoCodes => "tempoCodes",
            FrameKey::MusicCDIdentifier => "musicCdIdentifier",
            FrameKey::EventTimingCodes => "eventTimingCodes",
            FrameKey::Sequence => "sequence",
            FrameKey::PlayCount => "playCount",
            FrameKey::AudioSeekPointIndex => "audioSeekPointIndex",
            FrameKey::MediaType => "mediaType",
            FrameKey::CommercialFrame => "commercialFrame",
            FrameKey::AudioEncryption => "audioEncryption",
            FrameKey::SignatureFrame => "signatureFrame",
            FrameKey::SoftwareEncoder => "softwareEncoder",
            FrameKey::AudioEncodingMethod => "audioEncodingMethod",
            FrameKey::RecommendedBufferSize => "recommendedBufferSize",
            FrameKey::BeatsPerMinute => "beatsPerMinute",
            FrameKey::Language => "language",
            FrameKey::FileType => "fileType",
            FrameKey::Time => "time",
            FrameKey::RecordingDate => "recordingDate",
            FrameKey::ReleaseDate => "releaseDate",
        };
        write!(f, "{}", s)
    }
}

impl FrameKey {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "title" => Some(FrameKey::Title),
            "artist" => Some(FrameKey::Artist),
            "album" => Some(FrameKey::Album),
            "year" => Some(FrameKey::Year),
            "trackNumber" => Some(FrameKey::TrackNumber),
            "genre" => Some(FrameKey::Genre),
            "albumArtist" => Some(FrameKey::AlbumArtist),
            "acoustid_id" => Some(FrameKey::AcoustidId),
            "acoustid_fingerprint" => Some(FrameKey::AcoustidFingerprint),
            "albumartistsort" => Some(FrameKey::AlbumArtistSort),
            "albumsort" => Some(FrameKey::AlbumSort),
            "arranger" => Some(FrameKey::Arranger),
            "artistsort" => Some(FrameKey::ArtistSort),
            "artists" => Some(FrameKey::Artists),
            "asin" => Some(FrameKey::Asin),
            "barcode" => Some(FrameKey::Barcode),
            "catalognumber" => Some(FrameKey::CatalogNumber),
            "compilation" => Some(FrameKey::Compilation),
            "composersort" => Some(FrameKey::ComposerSort),
            "director" => Some(FrameKey::Director),
            "discnumber" => Some(FrameKey::DiscNumber),
            "discsubtitle" => Some(FrameKey::DiscSubtitle),
            "encodersettings" => Some(FrameKey::EncoderSettings),
            "engineer" => Some(FrameKey::Engineer),
            "gapless" => Some(FrameKey::Gapless),
            "grouping" => Some(FrameKey::Grouping),
            "key" => Some(FrameKey::InitialKey),
            "isrc" => Some(FrameKey::Isrc),
            "license" => Some(FrameKey::License),
            "lyricist" => Some(FrameKey::Lyricist),
            "lyrics" => Some(FrameKey::Lyrics),
            "media" => Some(FrameKey::Media),
            "mixer" => Some(FrameKey::Mixer),
            "mood" => Some(FrameKey::Mood),
            "movement" => Some(FrameKey::Movement),
            "movementtotal" => Some(FrameKey::MovementTotal),
            "movementnumber" => Some(FrameKey::MovementNumber),
            "musicbrainz_artistid" => Some(FrameKey::MusicBrainzArtistId),
            "musicbrainz_discid" => Some(FrameKey::MusicBrainzDiscId),
            "musicbrainz_originalartistid" => Some(FrameKey::MusicBrainzOriginalArtistId),
            "musicbrainz_originalalbumid" => Some(FrameKey::MusicBrainzOriginalAlbumId),
            "musicbrainz_recordingid" => Some(FrameKey::MusicBrainzRecordingId),
            "musicbrainz_albumartistid" => Some(FrameKey::MusicBrainzAlbumArtistId),
            "musicbrainz_releasegroupid" => Some(FrameKey::MusicBrainzReleaseGroupId),
            "musicbrainz_albumid" => Some(FrameKey::MusicBrainzAlbumId),
            "musicbrainz_trackid" => Some(FrameKey::MusicBrainzTrackId),
            "musicbrainz_releasetrackid" => Some(FrameKey::MusicBrainzReleaseTrackId),
            "musicbrainz_trmid" => Some(FrameKey::MusicBrainzTrmId),
            "musicbrainz_workid" => Some(FrameKey::MusicBrainzWorkId),
            "musicip_fingerprint" => Some(FrameKey::MusicIpFingerprint),
            "musicip_puid" => Some(FrameKey::MusicIpPuid),
            "originalalbum" => Some(FrameKey::OriginalAlbum),
            "originalartist" => Some(FrameKey::OriginalArtist),
            "originalfilename" => Some(FrameKey::OriginalFilename),
            "originaldate" => Some(FrameKey::OriginalDate),
            "originalyear" => Some(FrameKey::OriginalYear),
            "performer" => Some(FrameKey::Performer),
            "podcast" => Some(FrameKey::Podcast),
            "podcasturl" => Some(FrameKey::PodcastUrl),
            "producer" => Some(FrameKey::Producer),
            "_rating" => Some(FrameKey::Rating),
            "label" => Some(FrameKey::Label),
            "releasecountry" => Some(FrameKey::ReleaseCountry),
            "releasestatus" => Some(FrameKey::ReleaseStatus),
            "releasetype" => Some(FrameKey::ReleaseType),
            "remixer" => Some(FrameKey::Remixer),
            "replaygain_album_gain" => Some(FrameKey::ReplayGainAlbumGain),
            "replaygain_album_peak" => Some(FrameKey::ReplayGainAlbumPeak),
            "replaygain_album_range" => Some(FrameKey::ReplayGainAlbumRange),
            "replaygain_reference_loudness" => Some(FrameKey::ReplayGainReferenceLoudness),
            "replaygain_track_gain" => Some(FrameKey::ReplayGainTrackGain),
            "replaygain_track_peak" => Some(FrameKey::ReplayGainTrackPeak),
            "replaygain_track_range" => Some(FrameKey::ReplayGainTrackRange),
            "script" => Some(FrameKey::Script),
            "show" => Some(FrameKey::Show),
            "showsort" => Some(FrameKey::ShowSort),
            "showmovement" => Some(FrameKey::ShowMovement),
            "subtitle" => Some(FrameKey::Subtitle),
            "totaldiscs" => Some(FrameKey::TotalDiscs),
            "totaltracks" => Some(FrameKey::TotalTracks),
            "titlesort" => Some(FrameKey::TitleSort),
            "website" => Some(FrameKey::Website),
            "work" => Some(FrameKey::Work),
            "writer" => Some(FrameKey::Writer),
            "contentGroup" => Some(FrameKey::ContentGroup),
            "composer" => Some(FrameKey::Composer),
            "encodedBy" => Some(FrameKey::EncodedBy),
            "unsyncedLyrics" => Some(FrameKey::UnsyncedLyrics),
            "length" => Some(FrameKey::Length),
            "conductor" => Some(FrameKey::Conductor),
            "attachedPicture" => Some(FrameKey::AttachedPicture),
            "userDefinedUrl" => Some(FrameKey::UserDefinedURL),
            "comments" => Some(FrameKey::Comments),
            "private" => Some(FrameKey::Private),
            "relativeVolumeAdjustment" => Some(FrameKey::RelativeVolumeAdjustment),
            "encryptionMethod" => Some(FrameKey::EncryptionMethod),
            "groupIdRegistration" => Some(FrameKey::GroupIdRegistration),
            "generalObject" => Some(FrameKey::GeneralObject),
            "commercialUrl" => Some(FrameKey::CommercialURL),
            "copyrightUrl" => Some(FrameKey::CopyrightURL),
            "audioFileUrl" => Some(FrameKey::AudioFileURL),
            "artistUrl" => Some(FrameKey::ArtistURL),
            "radioStationUrl" => Some(FrameKey::RadioStationURL),
            "paymentUrl" => Some(FrameKey::PaymentURL),
            "bitmapImageUrl" => Some(FrameKey::BitmapImageURL),
            "userDefinedText" => Some(FrameKey::UserDefinedText),
            "synchronizedLyrics" => Some(FrameKey::SynchronizedLyrics),
            "tempoCodes" => Some(FrameKey::TempoCodes),
            "musicCdIdentifier" => Some(FrameKey::MusicCDIdentifier),
            "eventTimingCodes" => Some(FrameKey::EventTimingCodes),
            "sequence" => Some(FrameKey::Sequence),
            "playCount" => Some(FrameKey::PlayCount),
            "audioSeekPointIndex" => Some(FrameKey::AudioSeekPointIndex),
            "mediaType" => Some(FrameKey::MediaType),
            "commercialFrame" => Some(FrameKey::CommercialFrame),
            "audioEncryption" => Some(FrameKey::AudioEncryption),
            "signatureFrame" => Some(FrameKey::SignatureFrame),
            "softwareEncoder" => Some(FrameKey::SoftwareEncoder),
            "audioEncodingMethod" => Some(FrameKey::AudioEncodingMethod),
            "recommendedBufferSize" => Some(FrameKey::RecommendedBufferSize),
            "beatsPerMinute" => Some(FrameKey::BeatsPerMinute),
            "language" => Some(FrameKey::Language),
            "fileType" => Some(FrameKey::FileType),
            "time" => Some(FrameKey::Time),
            "recordingDate" => Some(FrameKey::RecordingDate),
            "releaseDate" => Some(FrameKey::ReleaseDate),
            _ => None,
        }
    }
}
// string to frame key
pub struct TagValuesWrapper(pub Vec<TagValue>);
pub struct SerializableTagValuesWrapper(pub Vec<SerializableTagValue>);

impl From<TagValuesWrapper> for Vec<SerializableTagValue> {
    fn from(values: TagValuesWrapper) -> Vec<SerializableTagValue> {
        let e: Vec<SerializableTagValue> = values
            .0
            .into_iter()
            .map(|value| match value {
                TagValue::Text(s) => SerializableTagValue::Text(s),
                TagValue::Picture {
                    mime,
                    data,
                    picture_type,
                    description,
                } => {
                    use base64::{engine::general_purpose::STANDARD, Engine as _};

                    let data_base64 = STANDARD.encode(data);

                    SerializableTagValue::Picture {
                        mime,
                        data_base64,
                        picture_type,
                        description,
                    }
                }
                TagValue::UserText(entry) => SerializableTagValue::UserText(entry),
                TagValue::UserUrl(entry) => SerializableTagValue::UserUrl(entry),
                TagValue::Comment {
                    encoding,
                    language,
                    description,
                    text,
                } => SerializableTagValue::Comment {
                    encoding,
                    language,
                    description,
                    text,
                },
            })
            .collect();
        e
    }
}

impl From<SerializableTagValuesWrapper> for Vec<TagValue> {
    fn from(values: SerializableTagValuesWrapper) -> Vec<TagValue> {
        let collected = values
            .0
            .iter()
            .map(|value| match value {
                SerializableTagValue::Text(s) => TagValue::Text(s.clone()),
                SerializableTagValue::Picture {
                    mime,
                    data_base64,
                    picture_type,
                    description,
                } => {
                    use base64::{engine::general_purpose::STANDARD, Engine as _};
                    let mime = mime.clone();

                    let data = STANDARD.decode(data_base64);
                    let data = data.unwrap_or_default();
                    let picture_type = picture_type.clone();
                    let description = description.clone();
                    TagValue::Picture {
                        mime,
                        data,
                        picture_type,
                        description,
                    }
                }
                SerializableTagValue::UserText(entry) => TagValue::UserText(entry.clone()),
                SerializableTagValue::UserUrl(entry) => TagValue::UserUrl(entry.clone()),
                SerializableTagValue::Comment {
                    encoding,
                    language,
                    description,
                    text,
                } => TagValue::Comment {
                    encoding: encoding.to_owned(),
                    language: language.to_owned(),
                    description: description.to_owned(),
                    text: text.to_owned(),
                },
            })
            .collect();
        collected
    }
}

/// Internal File struct to file that can be sent via ipc
impl From<MetadataFile> for SerializableFile {
    fn from(file: MetadataFile) -> Self {
        let mut tags: HashMap<String, Vec<SerializableTagValue>> = HashMap::new();
        for (k, vals) in file.tags.into_iter() {
            let key = k.to_string();
            let mut out_vals: Vec<SerializableTagValue> = Vec::new();

            for v in vals {
                match v {
                    TagValue::Text(s) => out_vals.push(SerializableTagValue::Text(s)),
                    TagValue::Picture {
                        mime,
                        data,
                        picture_type,
                        description,
                    } => out_vals.push(SerializableTagValue::Picture {
                        mime,
                        data_base64: general_purpose::STANDARD.encode(data),
                        picture_type,
                        description,
                    }),
                    TagValue::UserText(item) => out_vals.push(SerializableTagValue::UserText(item)),
                    TagValue::UserUrl(item) => out_vals.push(SerializableTagValue::UserUrl(item)),
                    TagValue::Comment {
                        encoding,
                        language,
                        description,
                        text,
                    } => out_vals.push(SerializableTagValue::Comment {
                        encoding,
                        language,
                        description,
                        text,
                    }),
                }
            }
            tags.insert(key, out_vals);
        }

        SerializableFile {
            path: file.path.display().to_string(),
            tag_format: file.tag_format.to_string(),
            file_name: file
                .path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            tag_formats: file
                .tag_formats
                .into_iter()
                .map(|f| f.to_string())
                .collect(),
            tags,
            freeforms: file
                .freeforms
                .into_iter()
                .map(|ff| SerializableFreeform {
                    mean: ff.mean,
                    name: ff.name,
                    value: ff.value,
                })
                .collect(),
        }
    }
}

/// Generates a temporary file path for a given target file.
pub fn temp_path_for(target: &Path) -> PathBuf {
    let mut p = target.to_path_buf();
    let file_name = target
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    p.set_file_name(format!(".{}.tmp", file_name));
    p
}

//// Replaces the target file with the temporary file.
pub fn replace_tmp(tmp: &Path, target: &Path) -> Result<(), ()> {
    #[cfg(not(windows))]
    {
        let ez = fs::remove_file(target);
        if ez.is_err() {
            return Err(());
        }
        let rename_result = fs::rename(tmp, target);
        if rename_result.is_err() {
            return Err(());
        }
        Ok(())
    }
    #[cfg(windows)]
    {
        let _ = fs::remove_file(target);
        let rename_result = fs::rename(tmp, target);
        if rename_result.is_err() {
            return Err(());
        }
        Ok(())
    }
}
