//
//  CloudArtworkCache.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import CryptoKit
import Foundation
import Observation
import UIKit

@Observable @MainActor
final class CloudArtworkCache {
  private(set) var images: [String: UIImage] = [:]
  private var directory: URL?

  func configure(directory: URL) throws {
    let folder = directory.appendingPathComponent("artwork", isDirectory: true)
    try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
    self.directory = folder
  }

  func key(for track: CloudTrack) -> String { "\(track.id):\(track.revision):\(track.sha256)" }
  func image(for track: CloudTrack) -> UIImage? { images[key(for: track)] }

  func restore(for track: CloudTrack) -> UIImage? {
    if let image = image(for: track) { return image }
    guard let file = file(for: track), let data = try? Data(contentsOf: file),
      let image = UIImage(data: data)
    else { return nil }
    images[key(for: track)] = image
    return image
  }

  func store(_ image: UIImage, data: Data, for track: CloudTrack) throws {
    images[key(for: track)] = image
    if let file = file(for: track) { try data.write(to: file, options: .atomic) }
  }

  private func file(for track: CloudTrack) -> URL? {
    let name = SHA256.hash(data: Data(key(for: track).utf8))
      .map { String(format: "%02x", $0) }.joined()
    return directory?.appendingPathComponent(name)
  }
}
