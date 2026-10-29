//
//  CloudPlayback.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import AVFAudio
import Foundation
import Observation

@Observable @MainActor
final class CloudPlayback {
  private(set) var track: CloudTrack?
  private(set) var isPlaying = false
  private(set) var isLoading = false
  private(set) var position: Double = 0
  private(set) var duration: Double = 0
  var error: String?
  private(set) var mediaControlsError: String?
  @ObservationIgnored var onNext: (() -> Void)?
  @ObservationIgnored var onPrevious: (() -> Void)?
  @ObservationIgnored var onStop: (() -> Void)?
  @ObservationIgnored var onResumeQueue: (() -> Void)?
  @ObservationIgnored private let core: MobileCore
  @ObservationIgnored private let audioSession = IOSAudioSession.shared
  private static var revisionCounter: UInt64 = 0
  @ObservationIgnored private var sessionRevision: UInt64 = 0
  @ObservationIgnored private var wantsPlayback = false
  @ObservationIgnored private var hasLoaded = false
  @ObservationIgnored private var completionHandled = false
  @ObservationIgnored private var preparedCommand: MobileCommand?
  @ObservationIgnored private var commandTask: Task<Void, Never>?
  @ObservationIgnored private var pollingTask: Task<Void, Never>?
  @ObservationIgnored private var notifications: [NSObjectProtocol] = []
  @ObservationIgnored private var resumeAfterInterruption = false

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
    self.track = track
    position = 0
    duration = Double(track.duration_ms ?? 0) / 1000
    error = nil
    mediaControlsError = nil
    isLoading = true
    isPlaying = false
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
    updateNowPlaying()
    startPolling()
  }

  func play(_ track: CloudTrack, localURL: URL?, media: CloudMedia?) async throws {
    guard self.track?.id == track.id else { return }
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
    Task {
      await stopped.value
      await audioSession.deactivate(revision: revision)
    }
  }
  private func updateNowPlaying() {
    guard let track else { return }
    var request = command("player_metadata")
    request.id = track.id
    enqueue(request)
  }

  func updateQueue() {
    if track != nil { enqueue(command("player_queue")) }
  }

  func updateArtwork(trackID: String, file: URL?) {
    guard track?.id == trackID, let file else { return }
    var request = command("player_artwork")
    request.id = trackID
    request.path = file.path
    enqueue(request)
  }

  private func handleRemote(_ action: MobileRemoteAction) {
    guard track != nil else { return }
    switch action.kind {
    case "play": resume()
    case "pause": pause()
    case "toggle": toggle()
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
    for token in notifications { NotificationCenter.default.removeObserver(token) }
    notifications.removeAll()
  }
}
