//
//  CloudModels.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import Foundation

nonisolated struct CloudMetadata: Codable, Sendable {
  var tags: [String: [String]]
  var loved: Bool
}

nonisolated struct CloudTrack: Codable, Identifiable, Sendable {
  let id: String
  let revision: Int64
  let sha256: String
  let size: Int64
  let file_name: String
  let format: String
  let duration_ms: Int64?
  let metadata: CloudMetadata
  let artwork: Bool
  let deleted: Bool

  var title: String { tag("title").isEmpty ? file_name : tag("title") }
  var artist: String { tag("artist").isEmpty ? "Unknown artist" : tag("artist") }
  var album: String { tag("album") }
  func tag(_ key: String) -> String { metadata.tags[key]?.first ?? "" }

  func playbackMIMEType(quality: String = "original", localURL: URL? = nil) -> String? {
    if quality == "aac_128" || quality == "aac_256" { return "audio/mp4" }
    let ext =
      (localURL?.pathExtension.isEmpty == false
      ? localURL!.pathExtension : (file_name as NSString).pathExtension).lowercased()
    switch ext {
    case "mp3": return "audio/mpeg"
    case "m4a", "mp4", "alac": return "audio/mp4"
    case "aac": return "audio/aac"
    case "wav", "wave": return "audio/wav"
    case "aif", "aiff", "aifc": return "audio/aiff"
    case "flac": return "audio/flac"
    default: return nil
    }
  }
}

nonisolated struct CloudPlaylist: Codable, Identifiable, Sendable {
  let id: String
  let revision: Int64
  let name: String
  let track_ids: [String]
  let deleted: Bool
}

nonisolated struct CloudChange: Codable, Sendable {
  struct Payload: Codable, Sendable {
    let kind: String
    let track: CloudTrack?
    let playlist: CloudPlaylist?
    enum CodingKeys: String, CodingKey { case kind, value }
    init(from decoder: Decoder) throws {
      let c = try decoder.container(keyedBy: CodingKeys.self)
      kind = try c.decode(String.self, forKey: .kind)
      switch kind {
      case "track":
        track = try c.decode(CloudTrack.self, forKey: .value)
        playlist = nil
      case "playlist":
        playlist = try c.decode(CloudPlaylist.self, forKey: .value)
        track = nil
      default: throw CloudError.message("Unknown cloud change type.")
      }
    }
    func encode(to encoder: Encoder) throws {
      var c = encoder.container(keyedBy: CodingKeys.self)
      try c.encode(kind, forKey: .kind)
      if let track { try c.encode(track, forKey: .value) }
      if let playlist { try c.encode(playlist, forKey: .value) }
    }
  }
  let cursor: Int64
  let value: Payload
}
nonisolated struct CloudChanges: Codable, Sendable {
  let changes: [CloudChange]
  let next_cursor: Int64
  let has_more: Bool
}
nonisolated struct CloudMedia: Codable, Sendable {
  let url: String
  let sha256: String
  let size: Int64
  let quality: String
  let preparing: Bool
}
nonisolated struct CloudIdentity: Decodable, Sendable {
  let authenticated: Bool
  let user_id: String
}
nonisolated struct MediaPlan: Decodable, Sendable {
  let path: String
  let quality: String
}
nonisolated struct CloudSnapshot: Decodable, Sendable {
  var cursor: Int64 = 0
  var tracks: [CloudTrack] = []
  var playlists: [CloudPlaylist] = []
  var queue: [String] = []
  var current: Int? = nil
  var downloads: [String: String] = [:]
  var currentTrack: CloudTrack? {
    guard let current, queue.indices.contains(current) else { return nil }
    return tracks.first { $0.id == queue[current] }
  }
}
nonisolated enum CloudError: LocalizedError {
  case message(String)
  var errorDescription: String? {
    switch self {
    case .message(let text): text
    }
  }
}

nonisolated struct MobileCommand: Encodable, Sendable {
  var op: String
  var directory: String? = nil
  var user_id: String? = nil
  var after: Int64? = nil
  var page: CloudChanges? = nil
  var ids: [String]? = nil
  var index: Int? = nil
  var id: String? = nil
  var next: Bool? = nil
  var previous: Bool? = nil
  var source_hash: String? = nil
  var temporary: String? = nil
  var media: CloudMedia? = nil
}
