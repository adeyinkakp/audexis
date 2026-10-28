//
//  CloudPlayback.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import AVFoundation
import Observation
import UIKit

@Observable @MainActor
final class CloudPlayback {
  private(set) var track: CloudTrack?
  private(set) var isPlaying = false
  private(set) var isLoading = false
  private(set) var position: Double = 0
  private(set) var duration: Double = 0
  var error: String?
  @ObservationIgnored var onNext: (() -> Void)?
  @ObservationIgnored var onPrevious: (() -> Void)?
  @ObservationIgnored var onStop: (() -> Void)?
  @ObservationIgnored var onResumeQueue: (() -> Void)?
  @ObservationIgnored private let audioSession = IOSAudioSession()
  @ObservationIgnored private var sessionRevision: UInt64 = 0
  @ObservationIgnored private var wantsPlayback = false
  @ObservationIgnored private let player = AVPlayer()
  @ObservationIgnored private var timeObserver: Any?
  @ObservationIgnored private var itemStatus: NSKeyValueObservation?
  @ObservationIgnored private var timeStatus: NSKeyValueObservation?
  @ObservationIgnored private var notifications: [NSObjectProtocol] = []
  @ObservationIgnored private lazy var mediaControls = IOSMediaControls(
    playback: self)
  @ObservationIgnored private var resumeAfterInterruption = false

  init() {
    timeObserver = player.addPeriodicTimeObserver(
      forInterval: CMTime(seconds: 0.5, preferredTimescale: 600), queue: .main
    ) { [weak self] time in
      Task { @MainActor [weak self] in
        guard let self else { return }
        self.position = time.seconds.isFinite ? time.seconds : 0
        let length = self.player.currentItem?.duration.seconds ?? .nan
        self.duration =
          length.isFinite ? length : Double(self.track?.duration_ms ?? 0) / 1000
        self.updateNowPlaying()
      }
    }
    timeStatus = player.observe(\.timeControlStatus, options: [.new]) { [weak self] _, _ in
      Task { @MainActor [weak self] in
        guard let self else { return }
        self.isPlaying = self.player.timeControlStatus == .playing
        if self.player.currentItem != nil {
          self.isLoading = self.player.timeControlStatus == .waitingToPlayAtSpecifiedRate
        }
        self.updateNowPlaying()
      }
    }
    observe(AVPlayerItem.didPlayToEndTimeNotification) { [weak self] note in
      guard let self, let item = note.object as? AVPlayerItem,
        item === self.player.currentItem
      else { return }
      self.onNext?()
    }
    observe(AVPlayerItem.failedToPlayToEndTimeNotification) { [weak self] note in
      guard let self, let item = note.object as? AVPlayerItem,
        item === self.player.currentItem
      else { return }
      self.error = "Playback was interrupted. Retry to reconnect to your music server."
      self.isLoading = false
    }
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
      guard let raw = note.userInfo?[AVAudioSessionRouteChangeReasonKey] as? UInt,
        AVAudioSession.RouteChangeReason(rawValue: raw) == .oldDeviceUnavailable
      else { return }
      self?.pause()
    }
    _ = mediaControls
  }
  private func observe(
    _ name: Notification.Name, handler: @escaping @MainActor (Notification) -> Void
  ) {
    notifications.append(
      NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) {
        note in
        MainActor.assumeIsolated { handler(note) }
      })
  }
  func preparing(_ track: CloudTrack) {
    sessionRevision &+= 1
    wantsPlayback = true
    player.pause()
    player.replaceCurrentItem(with: nil)
    self.track = track
    position = 0
    duration = Double(track.duration_ms ?? 0) / 1000
    error = nil
    isLoading = true
    isPlaying = false
    updateNowPlaying()
  }
  func restore(_ track: CloudTrack) {
    preparing(track)
    wantsPlayback = false
    isLoading = false
    updateNowPlaying()
  }
  func refreshMediaControls() {
    guard player.currentItem != nil else { return }
    mediaControls.activate()
    updateNowPlaying()
  }
  func play(_ track: CloudTrack, url: URL, mimeType: String?) async throws {
    let revision = sessionRevision
    try await audioSession.activate(revision: revision)
    try Task.checkCancellation()
    guard revision == sessionRevision, self.track?.id == track.id else { return }
    self.track = track

    var options: [String: Any] = [:]
    if let mimeType { options[AVURLAssetOverrideMIMETypeKey] = mimeType }
    let asset = AVURLAsset(url: url, options: options)
    let item = AVPlayerItem(asset: asset)
    itemStatus = item.observe(\.status, options: [.initial, .new]) { [weak self] item, _ in
      let failed = item.status == .failed
      let detail = item.error.map { error in
        let nsError = error as NSError
        let underlying = nsError.userInfo[NSUnderlyingErrorKey] as? NSError
        let reason = underlying?.localizedFailureReason ?? nsError.localizedFailureReason
        return [nsError.localizedDescription, reason].compactMap { $0 }.joined(
          separator: " ")
      }
      Task { @MainActor [weak self, weak item] in
        guard let self, let item, item === self.player.currentItem else { return }
        if failed {
          self.error =
            detail ?? "This audio could not be played. Try downloading it again."
          self.isLoading = false
        }
      }
    }
    player.replaceCurrentItem(with: item)
    if wantsPlayback { player.play() }
    mediaControls.activate()
    updateNowPlaying()
  }
  func toggle() { wantsPlayback ? pause() : resume() }
  func pause() {
    wantsPlayback = false
    player.pause()
    isPlaying = false
    updateNowPlaying()
  }
  func resume() {
    wantsPlayback = true
    guard player.currentItem != nil else {
      if track != nil { onResumeQueue?() }
      return
    }
    let revision = sessionRevision
    Task { [weak self] in
      guard let self else { return }
      do {
        try await self.audioSession.activate(revision: revision)
        guard revision == self.sessionRevision, self.wantsPlayback else { return }
        self.player.play()
        self.mediaControls.activate()
        self.updateNowPlaying()
      } catch {
        if revision == self.sessionRevision { self.error = error.localizedDescription }
      }
    }
  }
  func seek(_ seconds: Double) {
    guard seconds.isFinite else { return }
    let target = duration > 0 ? min(max(0, seconds), duration) : max(0, seconds)
    player.seek(to: CMTime(seconds: target, preferredTimescale: 600))
    position = target
    updateNowPlaying()
  }
  func stop() {
    sessionRevision &+= 1
    wantsPlayback = false
    let revision = sessionRevision
    player.pause()
    player.replaceCurrentItem(with: nil)
    itemStatus = nil
    track = nil
    position = 0
    isPlaying = false
    isLoading = false
    mediaControls.clear()
    Task { await audioSession.deactivate(revision: revision) }
  }
  private func updateNowPlaying() { mediaControls.updateNowPlaying() }

  func updateQueue(index: Int?, count: Int) {
    mediaControls.updateQueue(index: index, count: count)
  }

  func updateArtwork(trackID: String, image: UIImage) {
    mediaControls.setArtwork(trackID: trackID, image: image)
  }

  func tearDown() {
    stop()
    if let timeObserver {
      player.removeTimeObserver(timeObserver)
      self.timeObserver = nil
    }
    itemStatus = nil
    timeStatus = nil
    for token in notifications { NotificationCenter.default.removeObserver(token) }
    notifications.removeAll()
    mediaControls.tearDown()
  }
}
