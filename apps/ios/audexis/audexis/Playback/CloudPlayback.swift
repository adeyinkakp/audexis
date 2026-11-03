//
//  CloudPlayback.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

// will break up eventually

import AVFAudio
import Foundation
import MediaPlayer
import Observation
import UIKit

@Observable @MainActor
final class CloudPlayback {
  private(set) var track: CloudTrack?
  private(set) var isPlaying = false
  private(set) var isLoading = false
  private(set) var position: Double = 0
  private(set) var duration: Double = 0
  private(set) var audioQuality: String?
  var error: String?
  private(set) var mediaControlsError: String?
  @ObservationIgnored var onNext: (() -> Void)?
  @ObservationIgnored var onPrevious: (() -> Void)?
  @ObservationIgnored var onStop: (() -> Void)?
  @ObservationIgnored var onToggleFavorite: ((CloudTrack) -> Void)?
  @ObservationIgnored var onResumeQueue: (() -> Void)?
  @ObservationIgnored private let core: MobileCore
  @ObservationIgnored private let audioSession = IOSAudioSession.shared
  private static var revisionCounter: UInt64 = 0
  private static weak var systemPlayer: CloudPlayback?
  @ObservationIgnored private var sessionRevision: UInt64 = 0
  @ObservationIgnored private var wantsPlayback = false
  @ObservationIgnored private var hasLoaded = false
  @ObservationIgnored private var completionHandled = false
  @ObservationIgnored private var preparedCommand: MobileCommand?
  @ObservationIgnored private var commandTask: Task<Void, Never>?
  @ObservationIgnored private var pollingTask: Task<Void, Never>?
  @ObservationIgnored private var notifications: [NSObjectProtocol] = []
  @ObservationIgnored private var resumeAfterInterruption = false
  @ObservationIgnored private var remoteTargets: [(MPRemoteCommand, Any)] = []
  @ObservationIgnored private var queueIndex: Int?
  @ObservationIgnored private var queueCount = 0
  @ObservationIgnored private var systemArtwork: MPMediaItemArtwork?

  init(core: MobileCore) {
    self.core = core
    observe(AVAudioSession.didBecomeInactiveNotification) { [weak self] note in
      guard let self,
        let context = note.userInfo?[AVAudioSession.deactivationContextKey]
          as? AVAudioSession.DeactivationContext,
        context.interruptionContext != nil
      else { return }
      self.resumeAfterInterruption = self.wantsPlayback
      self.pause()
    }
    observe(AVAudioSession.resumptionRecommendationNotification) { [weak self] note in
      guard let self,
        let context = note.userInfo?[AVAudioSession.resumptionContextKey]
          as? AVAudioSession.ResumptionContext
      else { return }
      if self.resumeAfterInterruption && context.recommendation == .shouldResume {
        self.resume()
      }
      self.resumeAfterInterruption = false
    }
    observe(AVAudioSession.routeChangeNotification) { [weak self] note in
      guard let self else { return }
      if let raw = note.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt,
        AVAudioSession.RouteChangeReason(rawValue: raw) == .oldDeviceUnavailable
      {
        self.pause()
      }
      if self.hasLoaded { self.enqueue(self.command("player_rebuild")) }
    }
  }

  private func observe(
    _ name: Notification.Name, handler: @escaping @MainActor (Notification) -> Void,
  ) {
    notifications.append(
      NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) {
        note in
        MainActor.assumeIsolated { handler(note) }
      })
  }

  @discardableResult
  private func enqueue(_ command: MobileCommand) -> Task<Void, Never> {
    let previous = commandTask
    let revision = sessionRevision
    let task = Task { [weak self] in
      await previous?.value
      guard let self, self.sessionRevision == revision else { return }
      do {
        let _: MobilePlaybackState = try await self.core.call(command)
      } catch {
        guard self.sessionRevision == revision else { return }
        if ["player_metadata", "player_artwork", "player_queue"].contains(command.op) {
          self.mediaControlsError = error.localizedDescription
          return
        }
        self.error = error.localizedDescription
        self.isLoading = false
      }
    }
    commandTask = task
    return task
  }

  private func command(_ op: String) -> MobileCommand {
    var command = MobileCommand(op: op)
    command.playback_revision = sessionRevision
    return command
  }

  func preparing(_ track: CloudTrack) {
    Self.revisionCounter &+= 1
    sessionRevision = Self.revisionCounter
    pollingTask?.cancel()
    wantsPlayback = true
    hasLoaded = false
    completionHandled = false
    preparedCommand = nil
    enqueue(command("player_stop"))
    audioQuality = nil
    if self.track?.id != track.id { systemArtwork = nil }
    self.track = track
    position = 0
    duration = Double(track.duration_ms ?? 0) / 1000
    error = nil
    mediaControlsError = nil
    isLoading = true
    isPlaying = false
    registerRemoteCommands()
    updateNowPlaying()
    startPolling()
  }

  func restore(_ track: CloudTrack) {
    preparing(track)
    wantsPlayback = false
    isLoading = false
    updateNowPlaying()
  }

  func refreshMediaControls() {
    guard track != nil else { return }
    registerRemoteCommands()
    updateNowPlaying()
    startPolling()
  }

  func refreshTrack(_ latest: CloudTrack) {
    guard let current = track, current.id == latest.id,
      current.revision != latest.revision
        || current.metadata.tags != latest.metadata.tags
        || current.metadata.loved != latest.metadata.loved
    else { return }
    track = latest
    updateNowPlaying()
  }

  func play(_ track: CloudTrack, localURL: URL?, media: CloudMedia?) async throws {
    guard self.track?.id == track.id else { return }
    let sourceFormat = (track.file_name as NSString).pathExtension.uppercased()
    let originalQuality = sourceFormat.isEmpty ? "Original" : "\(sourceFormat) · Original"
    if let media {
      switch media.quality {
      case "aac_128": audioQuality = "AAC · 128 kbps"
      case "aac_256": audioQuality = "AAC · 256 kbps"
      default: audioQuality = originalQuality
      }
    } else if let localURL {
      // Download filenames contain the actual audio hash. Converted copies
      // have a different hash; older downloads do not retain their bitrate.
      audioQuality =
        localURL.deletingPathExtension().lastPathComponent == track.sha256
        ? originalQuality : "AAC"
    }
    var request = command("player_load")
    request.id = track.id
    request.path = localURL?.path
    request.media = media
    preparedCommand = request
    try await startPrepared(revision: sessionRevision)
  }

  private func startPrepared(revision: UInt64) async throws {
    guard wantsPlayback, !hasLoaded, let request = preparedCommand else {
      if !wantsPlayback { isLoading = false }
      return
    }
    try await audioSession.activate(revision: revision)
    try Task.checkCancellation()
    guard sessionRevision == revision, wantsPlayback, !hasLoaded else { return }
    hasLoaded = true
    await enqueue(request).value
    guard sessionRevision == revision else { return }
    startPolling()
  }

  private func startPolling() {
    pollingTask?.cancel()
    pollingTask = Task { [weak self] in
      while !Task.isCancelled {
        do { try await Task.sleep(for: .milliseconds(250)) } catch { return }
        guard let self else { return }
        do {
          let state: MobilePlaybackState = try await self.core.call(
            .init(op: "player_state"))
          guard !Task.isCancelled, state.revision == self.sessionRevision,
            state.track_id == self.track?.id
          else { continue }
          self.position = Double(state.position_ms) / 1000
          if state.duration_ms > 0 { self.duration = Double(state.duration_ms) / 1000 }
          self.isPlaying = state.is_playing && self.wantsPlayback
          self.isLoading = state.is_loading && self.wantsPlayback
          self.updateNowPlaying()
          if let error = state.error { self.error = error }
          if let error = state.controls_error { self.mediaControlsError = error }
          if state.remote_actions.contains(where: {
            ["next", "previous", "stop"].contains($0.kind)
          }) {
            self.completionHandled = true
          }
          for action in state.remote_actions { self.handleRemote(action) }
          if state.finished && !self.completionHandled {
            self.completionHandled = true
            self.onNext?()
          }
        } catch {
          guard !Task.isCancelled else { return }
          self.error = error.localizedDescription
          self.isLoading = false
          return
        }
      }
    }
  }

  func toggle() { wantsPlayback ? pause() : resume() }
  func pause() {
    wantsPlayback = false
    if hasLoaded { enqueue(command("player_pause")) }
    isPlaying = false
    updateNowPlaying()
  }
  func resume() {
    wantsPlayback = true
    guard hasLoaded || preparedCommand != nil else {
      if track != nil { onResumeQueue?() }
      return
    }
    let revision = sessionRevision
    Task { [weak self] in
      guard let self else { return }
      do {
        if !self.hasLoaded {
          try await self.startPrepared(revision: revision)
        } else {
          try await self.audioSession.activate(revision: revision)
          guard self.sessionRevision == revision, self.wantsPlayback else { return }
          self.enqueue(self.command("player_rebuild"))
          self.enqueue(self.command("player_resume"))
        }
      } catch {
        if self.sessionRevision == revision { self.error = error.localizedDescription }
      }
    }
  }
  func seek(_ seconds: Double) {
    guard seconds.isFinite, hasLoaded else { return }
    let target = duration > 0 ? min(max(0, seconds), duration) : max(0, seconds)
    var request = command("player_seek")
    request.position_ms = UInt64(min(target * 1000, Double(Int64.max)))
    enqueue(request)
    position = target
    updateNowPlaying()
  }
  func stop() {
    Self.revisionCounter &+= 1
    sessionRevision = Self.revisionCounter
    wantsPlayback = false
    hasLoaded = false
    preparedCommand = nil
    pollingTask?.cancel()
    let revision = sessionRevision
    let stopped = enqueue(command("player_stop"))
    track = nil
    position = 0
    duration = 0
    isPlaying = false
    isLoading = false
    audioQuality = nil
    systemArtwork = nil
    updateNowPlaying()
    Task {
      await stopped.value
      await audioSession.deactivate(revision: revision)
    }
  }
  private func updateNowPlaying() {
    guard Self.systemPlayer === self else { return }
    updateRemoteAvailability()
    let center = MPNowPlayingInfoCenter.default()
    guard let track else {
      center.nowPlayingInfo = nil
      return
    }
    var info: [String: Any] = [
      MPMediaItemPropertyTitle: track.title,
      MPMediaItemPropertyArtist: track.artist,
      MPMediaItemPropertyAlbumTitle: track.album,
      MPNowPlayingInfoPropertyElapsedPlaybackTime: position,
      MPNowPlayingInfoPropertyPlaybackRate: isPlaying ? 1.0 : 0.0,
      MPNowPlayingInfoPropertyDefaultPlaybackRate: 1.0,
      MPNowPlayingInfoPropertyIsLiveStream: false,
      MPNowPlayingInfoPropertyMediaType: MPNowPlayingInfoMediaType.audio.rawValue,
      MPNowPlayingInfoPropertyPlaybackQueueCount: queueCount,
    ]
    if duration > 0 { info[MPMediaItemPropertyPlaybackDuration] = duration }
    if let queueIndex { info[MPNowPlayingInfoPropertyPlaybackQueueIndex] = queueIndex }
    if let systemArtwork { info[MPMediaItemPropertyArtwork] = systemArtwork }
    center.nowPlayingInfo = info
  }

  func updateQueue(index: Int?, count: Int) {
    queueIndex = index
    queueCount = count
    updateNowPlaying()
  }

  func updateArtwork(trackID: String, file: URL?) {
    guard track?.id == trackID, let file,
      let image = UIImage(contentsOfFile: file.path)
    else { return }
    systemArtwork = MPMediaItemArtwork(boundsSize: image.size) { _ in image }
    updateNowPlaying()
  }

  private func registerRemoteCommands() {
    Self.systemPlayer = self
    guard remoteTargets.isEmpty else { return }
    let center = MPRemoteCommandCenter.shared()
    register(center.playCommand) { $0.resume() }
    register(center.pauseCommand) { $0.pause() }
    register(center.togglePlayPauseCommand) { $0.toggle() }
    register(center.stopCommand) { player in
      if let stop = player.onStop { stop() } else { player.stop() }
    }
    register(center.nextTrackCommand) { player in
      player.completionHandled = true
      player.onNext?()
    }
    register(center.previousTrackCommand) { player in
      player.completionHandled = true
      player.onPrevious?()
    }
    register(center.likeCommand) { player in
      if let track = player.track { player.onToggleFavorite?(track) }
    }
    center.likeCommand.localizedTitle = "Favorite"
    center.likeCommand.localizedShortTitle = "Favorite"
    let seek = center.changePlaybackPositionCommand
    let target = seek.addTarget { [weak self] event in
      guard let event = event as? MPChangePlaybackPositionCommandEvent,
        event.positionTime.isFinite
      else { return .commandFailed }
      let position = event.positionTime
      Task { @MainActor [weak self] in
        guard let self, Self.systemPlayer === self else { return }
        self.seek(position)
      }
      return .success
    }
    remoteTargets.append((seek, target))
    updateRemoteAvailability()
  }

  private func register(
    _ command: MPRemoteCommand,
    action: @escaping @MainActor (CloudPlayback) -> Void,
  ) {
    let target = command.addTarget { [weak self] _ in
      Task { @MainActor [weak self] in
        guard let self, Self.systemPlayer === self, self.track != nil else { return }
        action(self)
      }
      return .success
    }
    remoteTargets.append((command, target))
  }

  private func updateRemoteAvailability() {
    let center = MPRemoteCommandCenter.shared()
    let selected = track != nil
    center.playCommand.isEnabled = selected
    center.pauseCommand.isEnabled = selected
    center.togglePlayPauseCommand.isEnabled = selected
    center.stopCommand.isEnabled = selected
    center.nextTrackCommand.isEnabled = selected && queueIndex.map { $0 + 1 < queueCount } == true
    center.previousTrackCommand.isEnabled = selected
    center.changePlaybackPositionCommand.isEnabled = selected && hasLoaded && duration > 0
    center.likeCommand.isEnabled = selected && onToggleFavorite != nil
    center.likeCommand.isActive = track?.metadata.loved == true
  }

  private func handleRemote(_ action: MobileRemoteAction) {
    guard track != nil else { return }
    switch action.kind {
    case "play": resume()
    case "pause": pause()
    case "toggle": toggle()
    case "toggle_favorite":
      if let track { onToggleFavorite?(track) }
    case "stop":
      if let onStop { onStop() } else { stop() }
    case "next": onNext?()
    case "previous": onPrevious?()
    case "seek":
      if let position = action.position_ms { seek(Double(position) / 1000) }
    case "skip":
      if let offset = action.offset_ms { seek(position + Double(offset) / 1000) }
    default: break
    }
  }

  func tearDown() {
    stop()
    for (command, target) in remoteTargets { command.removeTarget(target) }
    remoteTargets.removeAll()
    if Self.systemPlayer === self { Self.systemPlayer = nil }
    for token in notifications { NotificationCenter.default.removeObserver(token) }
    notifications.removeAll()
  }

  isolated deinit {
    for (command, target) in remoteTargets { command.removeTarget(target) }
    for token in notifications { NotificationCenter.default.removeObserver(token) }
  }
}
