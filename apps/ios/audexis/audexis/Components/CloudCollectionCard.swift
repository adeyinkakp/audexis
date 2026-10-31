//
//  CloudCollectionCard.swift
//  audexis
//
//  Created by Kp on 10/26/26.
//

import SwiftUI

struct CloudCollectionCard: View {
  let title: String
  let subtitle: String
  let tracks: [CloudTrack]

  var body: some View {
    VStack(alignment: .leading, spacing: 9) {
      GeometryReader { space in
        if let track = tracks.first {
          CloudArtwork(track: track, size: space.size.width)
        } else {
          RoundedRectangle(cornerRadius: 12).fill(AudexisStyle.secondary)
            .overlay {
              Image(systemName: "music.note.list").font(.largeTitle).foregroundStyle(
                AudexisStyle.primary)
            }
        }
      }
      .aspectRatio(1, contentMode: .fit)
      Text(title).font(.subheadline.weight(.semibold)).lineLimit(2)
      Text(subtitle).font(.caption).foregroundStyle(.secondary).lineLimit(1)
    }
    .frame(maxWidth: .infinity, alignment: .topLeading)
    .contentShape(Rectangle())
    .contextMenu {
      if let first = tracks.first { CloudTrackActions(track: first, context: tracks) }
    }
  }
}

struct CloudCollectionGrid: View {
  let collections: [MusicCollection]
  var body: some View {
    LazyVGrid(
      columns: [GridItem(.adaptive(minimum: 140, maximum: 230), spacing: 18, alignment: .top)],
      alignment: .leading, spacing: 24,
    ) {
      ForEach(collections) { collection in
        NavigationLink {
          CloudTrackCollectionPage(
            title: collection.title, subtitle: collection.subtitle,
            trackIDs: collection.tracks.map(\.id), )
        } label: {
          CloudCollectionCard(
            title: collection.title, subtitle: collection.subtitle, tracks: collection.tracks, )
        }
        .buttonStyle(.plain)
      }
    }
  }
}
