//
//  CloudArtwork.swift
//  audexis
//
//  Created by Kp on 10/22/26.
//

import SwiftUI
import UIKit

struct CloudArtwork: View {
    @Environment(CloudLibrary.self) private var library
    let track: CloudTrack
    var size: CGFloat = 44
    @State private var image: UIImage?
    var body: some View {
        Group {
            if let image {
                Image(uiImage: image).resizable().scaledToFill()
            } else {
                RoundedRectangle(cornerRadius: 8).fill(AudexisStyle.secondary)
                    .overlay { Image(systemName: "music.note").foregroundStyle(.secondary) }
            }
        }
        .frame(width: size, height: size).clipShape(RoundedRectangle(cornerRadius: 8))
        .accessibilityHidden(true)
        .task(id: "\(library.artworkCache.scope):\(library.artworkCache.key(for: track))") {
            image = library.artworkCache.image(for: track)
            let loaded = await library.artworkImage(track)
            guard !Task.isCancelled else { return }
            image = loaded
        }
    }
}
