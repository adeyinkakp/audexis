use super::*;

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
    #[serde(alias = "discnumber")]
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
    #[serde(rename = "userDefinedUrl", alias = "userDefinedURL")]
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
        matches!(
            self,
            FrameKey::AttachedPicture
                | FrameKey::UserDefinedText
                | FrameKey::UserDefinedURL
                | FrameKey::Genre
                | FrameKey::Artist
                | FrameKey::AlbumArtist
                | FrameKey::Composer
                | FrameKey::Lyricist
                | FrameKey::Comments
        )
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
