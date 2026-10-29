//
//  CloudAPI.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import Foundation

enum CloudAPIConfiguration {
  static var origin: String {
    Bundle.main.object(forInfoDictionaryKey: "AudexisAPIOrigin") as? String
      ?? "http://localhost:3000"
  }
}

actor CloudAPI {
  private let origin: String
  private let credential: String
  private let session: URLSession
  private final class NoRedirect: NSObject, URLSessionTaskDelegate, @unchecked Sendable {
    nonisolated func urlSession(
      _ session: URLSession, task: URLSessionTask,
      willPerformHTTPRedirection response: HTTPURLResponse,
      newRequest request: URLRequest,
      completionHandler: @escaping (URLRequest?) -> Void,
    ) {
      completionHandler(nil)
    }
  }
  init(origin: String, credential: String, configuration: URLSessionConfiguration = .ephemeral) {
    self.origin = origin.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
    self.credential = credential
    let config = configuration
    config.timeoutIntervalForRequest = 30
    self.session = URLSession(configuration: config, delegate: NoRedirect(), delegateQueue: nil)
  }
  func get<T: Decodable & Sendable>(_ path: String, as type: T.Type = T.self) async throws -> T {
    guard path.hasPrefix("/api/"), let url = URL(string: origin + path) else {
      throw CloudError.message("The music server address is invalid.")
    }
    var request = URLRequest(url: url)
    request.setValue("Bearer \(credential)", forHTTPHeaderField: "Authorization")
    let (data, response) = try await session.data(for: request)
    guard let response = response as? HTTPURLResponse else {
      throw CloudError.message("The server did not respond.")
    }
    guard (200..<300).contains(response.statusCode) else {
      let message: String
      switch response.statusCode {
      case 401: message = "Your session expired. Sign out and sign in again."
      case 403: message = "Your account does not have access to this library."
      case 429, 503:
        message =
          "Cloud transfers are temporarily unavailable. Try again later; downloaded music is still available."
      default: message = "The music server returned an error (\(response.statusCode))."
      }
      throw CloudError.message(message)
    }
    guard data.count <= 8 * 1024 * 1024 else {
      throw CloudError.message("The cloud response is too large.")
    }
    return try JSONDecoder().decode(T.self, from: data)
  }
  func playableMedia(_ plan: MediaPlan) async throws -> CloudMedia {
    for attempt in 0..<30 {
      try Task.checkCancellation()
      let media: CloudMedia = try await get(plan.path)
      guard let url = URL(string: media.url), ["http", "https"].contains(url.scheme),
        url.user == nil, url.password == nil, media.size > 0
      else {
        throw CloudError.message("The server returned invalid audio information.")
      }
      if media.quality == plan.quality { return media }
      guard media.preparing else {
        throw CloudError.message(
          "The server could not provide the selected audio quality. Choose Original or try a different quality."
        )
      }
      if attempt < 29 { try await Task.sleep(for: .seconds(2)) }
    }
    throw CloudError.message("The server is still preparing this song. Try again later.")
  }
}
