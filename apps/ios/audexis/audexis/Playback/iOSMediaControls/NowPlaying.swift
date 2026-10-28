//
//  NowPlaying.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import MediaPlayer

extension IOSMediaControls {
  func updateNowPlaying() {
    guard connected, let playback, let track = playback.track else { return }
    if artworkID != track.id {
      artworkID = track.id
      artwork = nil
    }
    var info: [String: Any] = [
      MPMediaItemPropertyTitle: track.title,
      MPMediaItemPropertyArtist: track.artist,
      MPMediaItemPropertyAlbumTitle: track.album,
      MPMediaItemPropertyPlaybackDuration: max(0, playback.duration),
      MPNowPlayingInfoPropertyElapsedPlaybackTime: max(0, playback.position),
      MPNowPlayingInfoPropertyPlaybackRate: playback.isPlaying ? 1.0 : 0.0,
      MPNowPlayingInfoPropertyDefaultPlaybackRate: 1.0,
      MPNowPlayingInfoPropertyIsLiveStream: false,
      MPNowPlayingInfoPropertyExternalContentIdentifier: track.id,
      MPNowPlayingInfoPropertyMediaType: MPNowPlayingInfoMediaType.audio.rawValue,
      MPNowPlayingInfoPropertyPlaybackQueueCount: queueCount,
    ]
    if let queueIndex { info[MPNowPlayingInfoPropertyPlaybackQueueIndex] = queueIndex }
    if let artwork { info[MPMediaItemPropertyArtwork] = artwork }
    infoCenter.nowPlayingInfo = info
    updateCommands()
  }
}
