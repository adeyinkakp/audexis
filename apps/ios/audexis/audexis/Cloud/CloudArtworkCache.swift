//
//  CloudArtworkCache.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import CryptoKit
import Foundation
import Kingfisher
import Observation
import UIKit

@Observable @MainActor
final class CloudArtworkCache {
    private(set) var scope = ""
    @ObservationIgnored private var cache: ImageCache?
    @ObservationIgnored private var legacyDirectory: URL?

    func configure(directory: URL) throws {
        let cache = try ImageCache(
            name: "artwork-v2",
            cacheDirectoryURL: directory,
        )
        cache.memoryStorage.config.totalCostLimit = 32 * 1024 * 1024
        cache.memoryStorage.config.countLimit = 100
        cache.diskStorage.config.sizeLimit = 256 * 1024 * 1024
        cache.diskStorage.config.expiration = .never
        self.cache?.clearMemoryCache()
        self.cache = cache
        scope = directory.path
        legacyDirectory = directory.appendingPathComponent(
            "artwork",
            isDirectory: true,
        )
    }

    func key(for track: CloudTrack) -> String {
        "\(track.id):\(track.revision):\(track.sha256)"
    }

    func image(for track: CloudTrack) -> UIImage? {
        cache?.retrieveImageInMemoryCache(forKey: key(for: track))
    }

    func restore(for track: CloudTrack) async -> UIImage? {
        guard let cache else { return nil }
        let key = key(for: track)
        let legacyDirectory = self.legacyDirectory
        if let result = try? await cache.retrieveImage(
            forKey: key,
            options: [.backgroundDecode],
        ),
            let image = result.image
        {
            return image
        }
        guard let legacyDirectory else { return nil }
        let name = SHA256.hash(data: Data(key.utf8)).map {
            String(format: "%02x", $0)
        }.joined()
        let file = legacyDirectory.appendingPathComponent(name)
        let image = await Task.detached(priority: .utility) {
            guard let data = try? Data(contentsOf: file) else {
                return nil as UIImage?
            }
            return Self.decode(data)
        }.value
        guard let image else { return nil }
        try? await cache.store(image, forKey: key)
        return image
    }

    func store(data: Data, for track: CloudTrack) async throws -> UIImage {
        guard let cache else {
            throw CloudError.message("Artwork cache is unavailable.")
        }
        let key = key(for: track)
        let image = await Task.detached(priority: .utility) {
            Self.decode(data)
        }.value
        try Task.checkCancellation()
        guard let image else {
            throw CloudError.message(
                "The downloaded artwork is invalid or unsupported."
            )
        }
        try await cache.store(image, forKey: key)
        return image
    }

    func reset() {
        cache?.clearMemoryCache()
        cache = nil
        legacyDirectory = nil
        scope = ""
    }

    nonisolated private static func decode(_ data: Data) -> UIImage? {
        DownsamplingImageProcessor(size: CGSize(width: 1024, height: 1024))
            .process(
                item: .data(data),
                options: KingfisherParsedOptionsInfo([.scaleFactor(1)]),
            )
    }
}
