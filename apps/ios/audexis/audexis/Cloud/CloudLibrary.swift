//
//  CloudLibrary.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import CryptoKit
import Foundation
import OSLog
import Observation
import UIKit

@Observable @MainActor
final class CloudLibrary {
  private(set) var snapshot = CloudSnapshot() {
    didSet { playback.updateQueue() }
  }
  private(set) var isLoading = true
  private(set) var isRefreshing = false
  private(set) var downloads: [String: Double] = [:]
  let artworkCache = CloudArtworkCache()
  var error: String?
  private(set) var playback: CloudPlayback
  @ObservationIgnored private let core: MobileCore
  @ObservationIgnored private var api: CloudAPI?
  @ObservationIgnored private var directory: URL?
  @ObservationIgnored private var playbackTask: Task<Void, Never>?
  @ObservationIgnored private var downloadTasks: [String: Task<Void, Never>] = [:]
  @ObservationIgnored private var artworkTasks: [String: Task<Void, Never>] = [:]
  @ObservationIgnored private var importing = false
  @ObservationIgnored private var closed = false
  @ObservationIgnored private var lifecycle = UUID()
  @ObservationIgnored private var playbackGeneration = UUID()
  private static let logger = Logger(subsystem: "com.kp.audexis", category: "Artwork")

  init() {
    let core = MobileCore()
    self.core = core
    playback = CloudPlayback(core: core)
  }

  func start() async {
    if directory != nil {
      if playback.track == nil { restoreQueueSelection() }
      playback.refreshMediaControls()
      return
    }
    if closed {
      playback = CloudPlayback(core: core)
      closed = false
    }
    let lifecycle = self.lifecycle
    guard let session = SessionStore.read() else {
      isLoading = false
      return
    }
    do {
      let origin =
        session.connection.kind == .cloud
        ? CloudAPIConfiguration.origin : session.connection.url
      _ = try ServerConnection.customURL(origin)
      let scope = SHA256.hash(data: Data((origin + "\n" + session.credential).utf8)).map {
        String(format: "%02x", $0)
      }.joined()
      let base = try FileManager.default.url(
        for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil,
        create: true, )
      var folder = base.appendingPathComponent("CloudLibrary/\(scope)", isDirectory: true)
      try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
      var values = URLResourceValues()
      values.isExcludedFromBackup = true
      try folder.setResourceValues(values)
      directory = folder
      try artworkCache.configure(directory: folder)
      api = CloudAPI(origin: origin, credential: session.credential)
      let restored = try await core.openLibrary(at: folder)
      guard !closed, self.lifecycle == lifecycle else { return }
      snapshot = restored
      isLoading = false
      playback.onStop = { [weak self] in self?.stopPlayback() }
      playback.onNext = { [weak self] in self?.advance() }
      playback.onResumeQueue = { [weak self] in self?.prepareCurrent() }
      playback.onPrevious = { [weak self] in
        guard let self else { return }
        if self.playback.position > 3 {
          self.playback.seek(0)
        } else {
          self.advance(previous: true)
        }
      }
      restoreQueueSelection()
      CloudDownloads.shared.connect()
      await importDownloads()
      await refresh()
    } catch {
      guard !closed, self.lifecycle == lifecycle else { return }
      self.error = error.localizedDescription
      isLoading = false
      directory = nil
      api = nil
    }
  }
  private func restoreQueueSelection() {
    guard let track = snapshot.currentTrack else { return }
    playback.restore(track)
    if artworkCache.restore(for: track) != nil {
      playback.updateArtwork(trackID: track.id, file: artworkCache.file(for: track))
    }
  }

  func refresh() async {
    guard !closed, !isRefreshing, let api else { return }
    isRefreshing = true
    defer { isRefreshing = false }
    do {
      let identity: CloudIdentity = try await api.get("/api/session")
      guard identity.authenticated else {
        throw CloudError.message("Sign in again to load your cloud library.")
      }
      snapshot = try await core.call(.init(op: "identity", user_id: identity.user_id))
      while !closed {
        try Task.checkCancellation()
        let after = snapshot.cursor
        let page: CloudChanges = try await api.get("/api/sync/changes?after=\(after)")
        let playing = snapshot.currentTrack?.id
        snapshot = try await core.call(.init(op: "changes", after: after, page: page))
        if playing != nil && snapshot.currentTrack?.id != playing {
          playbackTask?.cancel()
          playback.stop()
        }
        if !page.has_more { break }
      }
      self.error = nil
    } catch is CancellationError {} catch {
      if !closed { self.error = error.localizedDescription }
    }
  }
  func play(_ track: CloudTrack, among tracks: [CloudTrack]) {
    guard let index = tracks.firstIndex(where: { $0.id == track.id }) else { return }
    changeQueue(.init(op: "play", ids: tracks.map(\.id), index: index), startPlayback: true)
  }
  func enqueue(_ track: CloudTrack, next: Bool) {
    changeQueue(.init(op: "enqueue", id: track.id, next: next))
  }
  func selectQueue(_ index: Int) {
    changeQueue(.init(op: "select", index: index), startPlayback: true)
  }
  func removeQueue(_ index: Int) {
    let current = snapshot.current
    changeQueue(.init(op: "remove_queue", index: index), startPlayback: current == index)
  }
  func clearQueue() { changeQueue(.init(op: "clear_queue"), startPlayback: true) }
  func advance(previous: Bool = false) {
    changeQueue(.init(op: "advance", previous: previous), startPlayback: true)
  }
  private func changeQueue(_ command: MobileCommand, startPlayback: Bool = false) {
    Task {
      guard !closed else { return }
      do {
        snapshot = try await core.call(command)
        if startPlayback { prepareCurrent() }
      } catch { self.error = error.localizedDescription }
    }
  }
  private func stopPlayback() {
    playbackGeneration = UUID()
    playbackTask?.cancel()
    playback.stop()
  }
  func retryPlayback() { prepareCurrent() }
  private func prepareCurrent() {
    playbackTask?.cancel()
    playbackGeneration = UUID()
    guard let track = snapshot.currentTrack, let api else {
      playback.stop()
      return
    }
    let generation = playbackGeneration
    playback.preparing(track)
    if artworkCache.image(for: track) != nil {
      playback.updateArtwork(trackID: track.id, file: artworkCache.file(for: track))
    }
    loadArtwork(track)
    let streamingQuality = AudioQualityPreferences.streaming
    playbackTask = Task {
      do {
        let localURL: URL?
        let media: CloudMedia?
        if let local = snapshot.downloads[track.id],
          FileManager.default.fileExists(atPath: local)
        {
          localURL = URL(fileURLWithPath: local)
          media = nil
        } else {
          let plan: MediaPlan = try await core.call(
            .init(op: "media_plan", id: track.id, quality: streamingQuality.rawValue))
          media = try await api.playableMedia(plan)
          localURL = nil
        }
        try Task.checkCancellation()
        guard !closed, playbackGeneration == generation else { return }
        try await playback.play(track, localURL: localURL, media: media)
      } catch is CancellationError {} catch {
        if !closed && playbackGeneration == generation {
          playback.error = error.localizedDescription
        }
      }
    }
  }
  func download(_ track: CloudTrack) {
    guard downloads[track.id] == nil, downloadTasks[track.id] == nil,
      snapshot.downloads[track.id] == nil, let directory, let api
    else { return }
    downloads[track.id] = 0
    loadArtwork(track)
    let downloadQuality = AudioQualityPreferences.download
    downloadTasks[track.id] = Task {
      defer { downloadTasks[track.id] = nil }
      do {
        let plan: MediaPlan = try await core.call(
          .init(op: "media_plan", id: track.id, quality: downloadQuality.rawValue))
        let media = try await api.playableMedia(plan)
        try Task.checkCancellation()
        guard !closed else { return }
        try CloudDownloads.shared.start(
          .init(
            id: track.id, sourceHash: track.sha256, directory: directory.path,
            media: media, ))
      } catch is CancellationError { downloads[track.id] = nil } catch {
        downloads[track.id] = nil
        if !closed { self.error = error.localizedDescription }
      }
    }
  }
  func cancelDownload(_ track: CloudTrack) {
    downloadTasks[track.id]?.cancel()
    Task {
      if let directory {
        await CloudDownloads.shared.cancel(id: track.id, directory: directory.path)
      }
      downloads[track.id] = nil
    }
  }
  func removeDownload(_ track: CloudTrack) {
    Task {
      do { snapshot = try await core.call(.init(op: "remove_download", id: track.id)) } catch {
        self.error = error.localizedDescription
      }
    }
  }
  func importDownloads() async {
    guard !closed, !importing, let directory else { return }
    importing = true
    defer { importing = false }

    do { snapshot = try await core.librarySnapshot() } catch {
      self.error = error.localizedDescription
      return
    }
    let active = await CloudDownloads.shared.active(in: directory.path)
    downloads = active.merging(downloadTasks.mapValues { _ in 0 }) { value, _ in value }
    let pending = directory.appendingPathComponent("pending", isDirectory: true)
    guard
      let files = try? FileManager.default.contentsOfDirectory(
        at: pending, includingPropertiesForKeys: nil, )
    else { return }
    for file in files where file.pathExtension == "json" {
      do {
        let receipt = try JSONDecoder().decode(
          CloudDownloadReceipt.self, from: Data(contentsOf: file), )
        guard receipt.job.directory == directory.path else { continue }
        defer {
          if let temporary = receipt.temporary {
            try? FileManager.default.removeItem(atPath: temporary)
          }
          try? FileManager.default.removeItem(at: file)
        }
        if let detail = receipt.error { throw CloudError.message(detail) }
        if let temporary = receipt.temporary {
          snapshot = try await core.call(
            .init(
              op: "commit_download", id: receipt.job.id,
              source_hash: receipt.job.sourceHash, temporary: temporary,
              media: receipt.job.media, ))
          if let track = snapshot.tracks.first(where: { $0.id == receipt.job.id }) {
            loadArtwork(track)
          }
        }
      } catch { self.error = error.localizedDescription }
    }
  }
  func loadArtwork(_ track: CloudTrack) {
    guard track.artwork, !closed else { return }
    if artworkCache.restore(for: track) != nil {
      playback.updateArtwork(trackID: track.id, file: artworkCache.file(for: track))
      return
    }
    let key = artworkCache.key(for: track)
    guard artworkTasks[key] == nil, let api else { return }
    artworkTasks[key] = Task {
      defer { artworkTasks[key] = nil }
      do {
        let media: CloudMedia = try await api.get("/api/tracks/\(track.id)/artwork")
        guard let url = URL(string: media.url), ["https", "http"].contains(url.scheme),
          url.user == nil, url.password == nil
        else {
          throw CloudError.message("The server returned an invalid artwork URL.")
        }
        let (data, response) = try await URLSession.shared.data(from: url)
        try Task.checkCancellation()
        guard !closed else { return }
        guard let response = response as? HTTPURLResponse,
          (200..<300).contains(response.statusCode)
        else {
          let status = (response as? HTTPURLResponse)?.statusCode ?? 0
          throw CloudError.message("Artwork download failed (HTTP \(status)).")
        }
        guard data.count <= 8 * 1024 * 1024, Int64(data.count) == media.size,
          SHA256.hash(data: data).map({ String(format: "%02x", $0) }).joined()
            == media.sha256,
          let image = UIImage(data: data)
        else {
          throw CloudError.message("The downloaded artwork is invalid or unsupported.")
        }
        try artworkCache.store(image, data: data, for: track)
        playback.updateArtwork(trackID: track.id, file: artworkCache.file(for: track))
      } catch is CancellationError {} catch {
        Self.logger.warning(
          "Artwork failed for track \(track.id, privacy: .public): \(error.localizedDescription, privacy: .public)"
        )
      }
    }
  }
  func shutdown() {
    guard !closed else { return }
    closed = true
    lifecycle = UUID()
    playbackGeneration = UUID()
    playbackTask?.cancel()
    for task in downloadTasks.values { task.cancel() }
    for task in artworkTasks.values { task.cancel() }
    downloadTasks.removeAll()
    artworkTasks.removeAll()
    playback.tearDown()
    directory = nil
    api = nil
    isLoading = true
  }
}
