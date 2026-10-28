//
//  IOSMediaControls.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import Foundation
import MediaPlayer

@MainActor
final class IOSMediaControls: NSObject {
  weak var playback: CloudPlayback?
  let center = MPRemoteCommandCenter.shared()
  let infoCenter = MPNowPlayingInfoCenter.default()
  var targets: [(MPRemoteCommand, Any)] = []
  var queueIndex: Int?
  var queueCount = 0
  var artworkID: String?
  var artwork: MPMediaItemArtwork?
  var connected = true

  init(playback: CloudPlayback) {
    self.playback = playback
    super.init()
    registerCommands()
    updateCommands()
  }

  func activate() {
    guard connected, playback?.track != nil else { return }
    updateNowPlaying()
  }

  func updateQueue(index: Int?, count: Int) {
    queueIndex = index
    queueCount = count
    updateCommands()
    updateNowPlaying()
  }

  func updateCommands() {
    let hasTrack = playback?.track != nil
    let playable = hasTrack && playback?.error == nil
    center.playCommand.isEnabled = playable
    center.pauseCommand.isEnabled = playable
    center.togglePlayPauseCommand.isEnabled = playable
    center.stopCommand.isEnabled = hasTrack
    center.nextTrackCommand.isEnabled =
      hasTrack && queueIndex.map { $0 + 1 < queueCount } == true
    center.previousTrackCommand.isEnabled = hasTrack && queueIndex != nil
    let canSeek = playable && (playback?.duration ?? 0) > 0
    center.changePlaybackPositionCommand.isEnabled = canSeek

  }

  func clear() {
    artworkID = nil
    artwork = nil
    infoCenter.nowPlayingInfo = nil
    updateCommands()
  }

  func tearDown() {
    connected = false
    clear()
    for (command, target) in targets {
      command.removeTarget(target)
      command.isEnabled = false
    }
    targets.removeAll()
    playback = nil
  }
}
