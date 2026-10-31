//
//  CloudDownloads.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import Foundation
import UIKit

nonisolated struct CloudDownloadJob: Codable, Sendable {
  let id: String
  let sourceHash: String
  let directory: String
  let media: CloudMedia
}
nonisolated struct CloudDownloadReceipt: Codable, Sendable {
  let job: CloudDownloadJob
  let temporary: String?
  let error: String?
}

nonisolated final class CloudDownloads: NSObject, URLSessionDownloadDelegate, @unchecked Sendable {
  static let shared = CloudDownloads()
  static let changed = Notification.Name("AudexisCloudDownloadsChanged")
  static let identifier = "com.kp.audexis.cloud-audio"
  @MainActor private lazy var session: URLSession = {
    let config = URLSessionConfiguration.background(withIdentifier: Self.identifier)
    config.sessionSendsLaunchEvents = true
    config.isDiscretionary = false
    config.httpMaximumConnectionsPerHost = 3
    return URLSession(configuration: config, delegate: self, delegateQueue: nil)
  }()
  @MainActor func connect() { _ = session }
  @MainActor var backgroundCompletion: (() -> Void)?

  @MainActor func start(_ job: CloudDownloadJob) throws {
    guard let url = URL(string: job.media.url) else {
      throw CloudError.message("Invalid download URL.")
    }
    let task = session.downloadTask(with: url)
    task.taskDescription = String(decoding: try JSONEncoder().encode(job), as: UTF8.self)
    task.resume()
  }
  @MainActor func active(in directory: String) async -> [String: Double] {
    let tasks = await session.allTasks
    return tasks.reduce(into: [:]) { result, task in
      guard let job = job(task), job.directory == directory else { return }
      result[job.id] =
        task.countOfBytesExpectedToReceive > 0
        ? min(
          1,
          Double(task.countOfBytesReceived) / Double(task.countOfBytesExpectedToReceive), )
        : 0
    }
  }
  @MainActor func cancel(id: String? = nil, directory: String) async {
    for task in await session.allTasks {
      if let job = job(task), job.directory == directory, id == nil || job.id == id {
        task.cancel()
      }
    }
  }
  private func job(_ task: URLSessionTask) -> CloudDownloadJob? {
    guard let text = task.taskDescription else { return nil }
    return try? JSONDecoder().decode(CloudDownloadJob.self, from: Data(text.utf8))
  }
  private func receipt(_ value: CloudDownloadReceipt, task: URLSessionTask) {
    let root = URL(fileURLWithPath: value.job.directory).appendingPathComponent(
      "pending", isDirectory: true, )
    do {
      try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
      let path = root.appendingPathComponent("\(task.taskIdentifier).json")
      if value.temporary != nil || value.error != nil
        || !FileManager.default.fileExists(atPath: path.path)
      {
        try JSONEncoder().encode(value).write(to: path, options: .atomic)
      }
    } catch {
      NotificationCenter.default.post(
        name: Self.changed, object: nil,
        userInfo: [
          "error": "Could not save the downloaded song: \(error.localizedDescription)"
        ], )
    }
    NotificationCenter.default.post(name: Self.changed, object: nil)
  }
  func urlSession(
    _ session: URLSession, downloadTask: URLSessionDownloadTask,
    didFinishDownloadingTo location: URL,
  ) {
    guard let job = job(downloadTask) else { return }
    guard let response = downloadTask.response as? HTTPURLResponse,
      response.statusCode == 200
    else {
      receipt(
        .init(
          job: job, temporary: nil,
          error: "Download failed. Try downloading the song again.", ), task: downloadTask, )
      return
    }
    do {
      let staging = URL(fileURLWithPath: job.directory).appendingPathComponent(
        "pending", isDirectory: true, )
      try FileManager.default.createDirectory(at: staging, withIntermediateDirectories: true)
      let file = staging.appendingPathComponent("\(UUID().uuidString).audio")
      try FileManager.default.moveItem(at: location, to: file)
      receipt(.init(job: job, temporary: file.path, error: nil), task: downloadTask)
    } catch {
      receipt(
        .init(job: job, temporary: nil, error: error.localizedDescription),
        task: downloadTask, )
    }
  }
  func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
    if let error, let job = job(task) {
      let cancelled = (error as NSError).code == NSURLErrorCancelled
      receipt(
        .init(
          job: job, temporary: nil, error: cancelled ? nil : error.localizedDescription, ),
        task: task, )
    }
    NotificationCenter.default.post(name: Self.changed, object: nil)
  }
  func urlSession(
    _ session: URLSession, downloadTask: URLSessionDownloadTask,
    didWriteData bytesWritten: Int64, totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64,
  ) {
    if let job = job(downloadTask), totalBytesWritten > job.media.size {
      downloadTask.cancel()
      receipt(
        .init(
          job: job, temporary: nil,
          error: "The downloaded audio exceeds its declared size.", ), task: downloadTask, )
    }
    NotificationCenter.default.post(name: Self.changed, object: nil)
  }
  func urlSessionDidFinishEvents(forBackgroundURLSession session: URLSession) {
    Task { @MainActor in
      backgroundCompletion?()
      backgroundCompletion = nil
    }
  }
}

final class AudexisAppDelegate: NSObject, UIApplicationDelegate {
  func application(
    _ application: UIApplication, handleEventsForBackgroundURLSession identifier: String,
    completionHandler: @escaping () -> Void,
  ) {
    guard identifier == CloudDownloads.identifier else {
      completionHandler()
      return
    }
    CloudDownloads.shared.backgroundCompletion = completionHandler
    CloudDownloads.shared.connect()
  }
}
