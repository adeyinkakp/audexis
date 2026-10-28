//
//  Rewind.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct RewindPage: View {
  @Environment(CloudLibrary.self) private var library
  private var favorites: [CloudTrack] { library.snapshot.tracks.filter { $0.metadata.loved } }

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 28) {
        VStack(alignment: .leading, spacing: 16) {
          Image(systemName: "clock.arrow.circlepath").font(.system(size: 44))
            .foregroundStyle(AudexisStyle.primary)
          Text("Ya history").font(.largeTitle.bold())
          Text(
            "Rewind Not available."
          )
          .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, alignment: .leading).padding(24)
        .background(AudexisStyle.card, in: RoundedRectangle(cornerRadius: 24))
        if !favorites.isEmpty {
          Text("Your favorites").font(.title2.bold())
          CloudCollectionActions(tracks: favorites)
          LazyVStack(spacing: 10) {
            ForEach(favorites) { CloudTrackRow(track: $0, context: favorites) }
          }
        }
      }
      .padding(20).frame(maxWidth: 1100).frame(maxWidth: .infinity)
    }
  }
}
