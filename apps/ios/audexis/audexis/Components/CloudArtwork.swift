//
//  CloudArtwork.swift
//  audexis
//
//  Created by Kp on 10/22/26.
//

import SwiftUI

struct CloudArtwork: View {
  @Environment(CloudLibrary.self) private var library
  let track: CloudTrack
  var size: CGFloat = 44
  var body: some View {
    Group {
      if let image = library.artworkCache.image(for: track) {
        Image(uiImage: image).resizable().scaledToFill()
      } else {
        RoundedRectangle(cornerRadius: 8).fill(AudexisStyle.secondary)
          .overlay { Image(systemName: "music.note").foregroundStyle(.secondary) }
      }
    }
    .frame(width: size, height: size).clipShape(RoundedRectangle(cornerRadius: 8))
    .accessibilityHidden(true)
    .task(id: library.artworkCache.key(for: track)) { library.loadArtwork(track) }
  }
}
